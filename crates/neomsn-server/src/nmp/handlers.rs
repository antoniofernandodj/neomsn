use anyhow::Result;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tracing::warn;
use uuid::Uuid;
use neomsn_shared::proto::{
    Frame, Opcode,
    payload::{
        self, AuthFail, AuthOk, ContactAcceptOk, ContactAddOk, ContactEntry, ContactListResp,
        ContactRequest, ContextType, DmOpenResp, MsgChunk, MsgComplete, MsgDelete,
        PresenceStatus as PayloadPresence, PresenceUpdate, ProfileResp, RoomListResp,
    },
};
use crate::{
    auth::verify_token,
    db::entities::{
        contact, direct_conversation, message, message_chunk, room, room_member, user,
    },
    state::SharedState,
};
use super::session::SessionState;

pub async fn dispatch(
    frame: Frame,
    session: &mut SessionState,
    state: &SharedState,
) -> Result<()> {
    // Before auth, only HELLO and AUTH are accepted.
    if !session.is_authenticated() {
        match frame.opcode {
            Opcode::Hello => { /* version negotiation — nothing to do yet */ }
            Opcode::Auth  => handle_auth(frame, session, state).await?,
            Opcode::Ping  => session.send(Frame::new(Opcode::Pong, vec![])).await,
            _ => {
                warn!("Received {:?} before AUTH — dropping", frame.opcode);
                session.send(Frame::new(
                    Opcode::Error,
                    payload::Error { code: 401, message: "not authenticated".into() }.encode(),
                )).await;
            }
        }
        return Ok(());
    }

    match frame.opcode {
        Opcode::Ping          => session.send(Frame::new(Opcode::Pong, vec![])).await,
        Opcode::MsgChunk      => handle_msg_chunk(frame, session, state).await?,
        Opcode::MsgComplete   => handle_msg_complete(frame, session, state).await?,
        Opcode::MsgDelete     => handle_msg_delete(frame, session, state).await?,
        Opcode::ContactList   => handle_contact_list(frame, session, state).await?,
        Opcode::ContactAdd    => handle_contact_add(frame, session, state).await?,
        Opcode::ContactRemove => handle_contact_remove(frame, session, state).await?,
        Opcode::ContactBlock  => handle_contact_block(frame, session, state).await?,
        Opcode::ContactAccept => handle_contact_accept(frame, session, state).await?,
        Opcode::ContactReject => handle_contact_reject(frame, session, state).await?,
        Opcode::DmOpen        => handle_dm_open(frame, session, state).await?,
        Opcode::RoomList      => handle_room_list(frame, session, state).await?,
        Opcode::RoomJoin      => handle_room_join(frame, session, state).await?,
        Opcode::RoomLeave     => handle_room_leave(frame, session, state).await?,
        Opcode::PresenceSet   => handle_presence_set(frame, session, state).await?,
        Opcode::ProfileGet    => handle_profile_get(frame, session, state).await?,
        Opcode::ProfileUpdate => handle_profile_update(frame, session, state).await?,
        other => warn!("Unhandled opcode: {other:?}"),
    }
    Ok(())
}

// ─── AUTH ────────────────────────────────────────────────────────────────────

async fn handle_auth(frame: Frame, session: &mut SessionState, state: &SharedState) -> Result<()> {
    let p = payload::Auth::decode(&frame.payload)
        .map_err(|_| anyhow::anyhow!("decode error"))?;

    let claims = match verify_token(&p.token) {
        Ok(c) => c,
        Err(_) => {
            session.send(Frame::new(
                Opcode::AuthFail,
                AuthFail { reason: "invalid token".into() }.encode(),
            )).await;
            return Ok(());
        }
    };

    let user_id = Uuid::parse_str(&claims.sub)?;
    let device_id = Uuid::parse_str(&claims.device_id)?;

    let user = user::Entity::find_by_id(user_id)
        .one(&state.db).await?
        .ok_or_else(|| anyhow::anyhow!("user not found"))?;

    let handle = session.authenticate(user_id, device_id);
    state.add_session(handle).await;

    // Mark online
    state.presence.write().await
        .insert(user_id, neomsn_shared::proto::payload::PresenceStatus::Online);

    session.send(Frame::new(
        Opcode::AuthOk,
        AuthOk {
            user_id,
            display_name: user.display_name.clone(),
            personal_message: user.personal_message.clone(),
        }.encode(),
    )).await;

    // Notify accepted contacts that this user came online.
    let contacts = contact::Entity::find()
        .filter(contact::Column::OwnerId.eq(user_id))
        .filter(contact::Column::State.eq("accepted"))
        .all(&state.db).await?;

    let update_frame = Frame::new(
        Opcode::PresenceUpdate,
        PresenceUpdate {
            user_id,
            status: neomsn_shared::proto::payload::PresenceStatus::Online,
        }.encode(),
    );
    for c in contacts {
        state.send_to_user(c.contact_id, update_frame.clone()).await;
    }

    Ok(())
}

// ─── MSG_CHUNK ───────────────────────────────────────────────────────────────

async fn handle_msg_chunk(frame: Frame, session: &mut SessionState, state: &SharedState) -> Result<()> {
    let p = MsgChunk::decode(&frame.payload).map_err(|_| anyhow::anyhow!("decode"))?;
    let user_id = session.user_id.unwrap();

    // Upsert message row (created on first chunk).
    let exists = message::Entity::find_by_id(p.msg_id).one(&state.db).await?.is_some();
    if !exists {
        let now = Utc::now();
        let ctx = match p.context_type { ContextType::Room => "room", ContextType::Dm => "dm" };
        message::ActiveModel {
            id: Set(p.msg_id),
            context_type: Set(ctx.into()),
            context_id: Set(p.context_id),
            author_id: Set(user_id),
            content: Set(String::new()),
            status: Set("streaming".into()),
            started_at: Set(now),
            completed_at: Set(None),
        }.insert(&state.db).await?;
    }

    // Persist chunk.
    use sea_orm::PaginatorTrait;
    let seq = message_chunk::Entity::find()
        .filter(message_chunk::Column::MessageId.eq(p.msg_id))
        .count(&state.db).await? as i32;

    message_chunk::ActiveModel {
        id: Default::default(),
        message_id: Set(p.msg_id),
        delta: Set(p.delta.clone()),
        seq: Set(seq),
        created_at: Set(Utc::now()),
    }.insert(&state.db).await?;

    // Broadcast to room/DM participants (excluding sender).
    broadcast_to_context(&p.context_type, p.context_id, user_id, frame, state).await;
    Ok(())
}

// ─── MSG_COMPLETE ─────────────────────────────────────────────────────────────

async fn handle_msg_complete(frame: Frame, session: &mut SessionState, state: &SharedState) -> Result<()> {
    let p = MsgComplete::decode(&frame.payload).map_err(|_| anyhow::anyhow!("decode"))?;
    let user_id = session.user_id.unwrap();

    // Update message: set content and status.
    if let Some(msg) = message::Entity::find_by_id(p.msg_id).one(&state.db).await? {
        let mut am: message::ActiveModel = msg.into();
        am.content = Set(p.content.clone());
        am.status = Set("complete".into());
        am.completed_at = Set(Some(Utc::now()));
        am.update(&state.db).await?;
    }

    broadcast_to_context(&p.context_type, p.context_id, user_id, frame, state).await;
    Ok(())
}

// ─── MSG_DELETE ───────────────────────────────────────────────────────────────

async fn handle_msg_delete(frame: Frame, session: &mut SessionState, state: &SharedState) -> Result<()> {
    let p = MsgDelete::decode(&frame.payload).map_err(|_| anyhow::anyhow!("decode"))?;
    let user_id = session.user_id.unwrap();

    if let Some(msg) = message::Entity::find_by_id(p.msg_id).one(&state.db).await? {
        let mut am: message::ActiveModel = msg.into();
        am.status = Set("deleted".into());
        am.update(&state.db).await?;
    }

    broadcast_to_context(&p.context_type, p.context_id, user_id, frame, state).await;
    Ok(())
}

// ─── CONTACT_LIST ────────────────────────────────────────────────────────────

async fn handle_contact_list(_frame: Frame, session: &mut SessionState, state: &SharedState) -> Result<()> {
    let user_id = session.user_id.unwrap();

    let contacts = contact::Entity::find()
        .filter(contact::Column::OwnerId.eq(user_id))
        .filter(contact::Column::State.eq("accepted"))
        .all(&state.db).await?;

    let presence_map = state.presence.read().await;
    let mut entries = Vec::new();

    for c in contacts {
        if let Some(u) = user::Entity::find_by_id(c.contact_id).one(&state.db).await? {
            let presence = *presence_map.get(&c.contact_id)
                .unwrap_or(&neomsn_shared::proto::payload::PresenceStatus::Offline);
            entries.push(ContactEntry {
                user_id: c.contact_id,
                username: u.username,
                display_name: u.display_name,
                presence,
            });
        }
    }

    session.send(Frame::new(
        Opcode::ContactListResp,
        ContactListResp { contacts: entries }.encode(),
    )).await;
    Ok(())
}

// ─── CONTACT_ADD ─────────────────────────────────────────────────────────────

async fn handle_contact_add(frame: Frame, session: &mut SessionState, state: &SharedState) -> Result<()> {
    let p = payload::ContactAdd::decode(&frame.payload).map_err(|_| anyhow::anyhow!("decode"))?;
    let owner_id = session.user_id.unwrap();

    let target = match user::Entity::find()
        .filter(user::Column::Username.eq(&p.username))
        .one(&state.db).await? {
        Some(u) => u,
        None => {
            session.send(Frame::new(Opcode::Error,
                payload::Error { code: 404, message: "user not found".into() }.encode())).await;
            return Ok(());
        }
    };

    // Create pending entry for owner.
    let exists = contact::Entity::find()
        .filter(contact::Column::OwnerId.eq(owner_id))
        .filter(contact::Column::ContactId.eq(target.id))
        .one(&state.db).await?.is_some();

    if !exists {
        contact::ActiveModel {
            owner_id: Set(owner_id),
            contact_id: Set(target.id),
            state: Set("pending".into()),
            since: Set(Utc::now()),
        }.insert(&state.db).await?;
    }

    // Notify the target that a request was sent.
    let requester = user::Entity::find_by_id(owner_id).one(&state.db).await?.unwrap();
    state.send_to_user(target.id, Frame::new(
        Opcode::ContactRequest,
        ContactRequest {
            user_id: owner_id,
            username: requester.username,
            display_name: requester.display_name,
        }.encode(),
    )).await;

    session.send(Frame::new(
        Opcode::ContactAddOk,
        ContactAddOk { user_id: target.id, username: target.username, display_name: target.display_name }.encode(),
    )).await;
    Ok(())
}

// ─── CONTACT_REMOVE ──────────────────────────────────────────────────────────

async fn handle_contact_remove(frame: Frame, session: &mut SessionState, state: &SharedState) -> Result<()> {
    let p = payload::ContactUserId::decode(&frame.payload).map_err(|_| anyhow::anyhow!("decode"))?;
    let owner_id = session.user_id.unwrap();

    contact::Entity::delete_many()
        .filter(contact::Column::OwnerId.eq(owner_id))
        .filter(contact::Column::ContactId.eq(p.user_id))
        .exec(&state.db).await?;
    Ok(())
}

// ─── CONTACT_BLOCK ───────────────────────────────────────────────────────────

async fn handle_contact_block(frame: Frame, session: &mut SessionState, state: &SharedState) -> Result<()> {
    let p = payload::ContactUserId::decode(&frame.payload).map_err(|_| anyhow::anyhow!("decode"))?;
    let owner_id = session.user_id.unwrap();

    if let Some(c) = contact::Entity::find()
        .filter(contact::Column::OwnerId.eq(owner_id))
        .filter(contact::Column::ContactId.eq(p.user_id))
        .one(&state.db).await? {
        let mut am: contact::ActiveModel = c.into();
        am.state = Set("blocked".into());
        am.update(&state.db).await?;
    } else {
        contact::ActiveModel {
            owner_id: Set(owner_id),
            contact_id: Set(p.user_id),
            state: Set("blocked".into()),
            since: Set(Utc::now()),
        }.insert(&state.db).await?;
    }
    Ok(())
}

// ─── CONTACT_ACCEPT ──────────────────────────────────────────────────────────

async fn handle_contact_accept(frame: Frame, session: &mut SessionState, state: &SharedState) -> Result<()> {
    let p = payload::ContactUserId::decode(&frame.payload).map_err(|_| anyhow::anyhow!("decode"))?;
    let self_id = session.user_id.unwrap();
    let requester_id = p.user_id;

    // Verify there actually is a pending request from requester → self.
    let pending = contact::Entity::find()
        .filter(contact::Column::OwnerId.eq(requester_id))
        .filter(contact::Column::ContactId.eq(self_id))
        .filter(contact::Column::State.eq("pending"))
        .one(&state.db).await?;

    if pending.is_none() {
        session.send(Frame::new(Opcode::Error,
            payload::Error { code: 404, message: "no pending request from that user".into() }.encode()
        )).await;
        return Ok(());
    }

    let now = Utc::now();

    // Mark requester→self as accepted (update existing row).
    let mut am: contact::ActiveModel = pending.unwrap().into();
    am.state = Set("accepted".into());
    am.since = Set(now);
    am.update(&state.db).await?;

    // Create or update self→requester as accepted.
    let self_to_req = contact::Entity::find()
        .filter(contact::Column::OwnerId.eq(self_id))
        .filter(contact::Column::ContactId.eq(requester_id))
        .one(&state.db).await?;

    if let Some(existing) = self_to_req {
        let mut am: contact::ActiveModel = existing.into();
        am.state = Set("accepted".into());
        am.since = Set(now);
        am.update(&state.db).await?;
    } else {
        contact::ActiveModel {
            owner_id: Set(self_id),
            contact_id: Set(requester_id),
            state: Set("accepted".into()),
            since: Set(now),
        }.insert(&state.db).await?;
    }

    let presence_map = state.presence.read().await;

    // Notify self: the requester is now a contact.
    let requester_user = user::Entity::find_by_id(requester_id).one(&state.db).await?
        .ok_or_else(|| anyhow::anyhow!("requester not found"))?;
    let req_presence = *presence_map.get(&requester_id).unwrap_or(&PayloadPresence::Offline);

    session.send(Frame::new(Opcode::ContactAcceptOk, ContactAcceptOk {
        user_id: requester_id,
        username: requester_user.username.clone(),
        display_name: requester_user.display_name.clone(),
        presence: req_presence,
    }.encode())).await;

    // Notify requester: their request was accepted, self is now their contact too.
    let self_user = user::Entity::find_by_id(self_id).one(&state.db).await?
        .ok_or_else(|| anyhow::anyhow!("self not found"))?;
    let self_presence = *presence_map.get(&self_id).unwrap_or(&PayloadPresence::Offline);

    state.send_to_user(requester_id, Frame::new(Opcode::ContactAcceptOk, ContactAcceptOk {
        user_id: self_id,
        username: self_user.username,
        display_name: self_user.display_name,
        presence: self_presence,
    }.encode())).await;

    Ok(())
}

// ─── CONTACT_REJECT ───────────────────────────────────────────────────────────

async fn handle_contact_reject(frame: Frame, session: &mut SessionState, state: &SharedState) -> Result<()> {
    let p = payload::ContactUserId::decode(&frame.payload).map_err(|_| anyhow::anyhow!("decode"))?;
    let self_id = session.user_id.unwrap();

    // Delete the pending request. The requester is not notified.
    contact::Entity::delete_many()
        .filter(contact::Column::OwnerId.eq(p.user_id))
        .filter(contact::Column::ContactId.eq(self_id))
        .filter(contact::Column::State.eq("pending"))
        .exec(&state.db).await?;

    Ok(())
}

// ─── DM_OPEN ─────────────────────────────────────────────────────────────────

async fn handle_dm_open(frame: Frame, session: &mut SessionState, state: &SharedState) -> Result<()> {
    let p = payload::DmOpen::decode(&frame.payload).map_err(|_| anyhow::anyhow!("decode"))?;
    let self_id = session.user_id.unwrap();

    let target = match user::Entity::find()
        .filter(user::Column::Username.eq(&p.username))
        .one(&state.db).await? {
        Some(u) => u,
        None => {
            session.send(Frame::new(Opcode::Error,
                payload::Error { code: 404, message: "user not found".into() }.encode())).await;
            return Ok(());
        }
    };

    // Canonical DM id: deterministic from both user UUIDs.
    let (a, b) = if self_id < target.id { (self_id, target.id) } else { (target.id, self_id) };

    let conv = if let Some(existing) = direct_conversation::Entity::find()
        .filter(direct_conversation::Column::UserA.eq(a))
        .filter(direct_conversation::Column::UserB.eq(b))
        .one(&state.db).await? {
        existing
    } else {
        direct_conversation::ActiveModel {
            id: Set(Uuid::new_v4()),
            user_a: Set(a),
            user_b: Set(b),
            created_at: Set(Utc::now()),
        }.insert(&state.db).await?
    };

    session.send(Frame::new(
        Opcode::DmOpenResp,
        DmOpenResp {
            conversation_id: conv.id,
            user_id: target.id,
            display_name: target.display_name,
        }.encode(),
    )).await;
    Ok(())
}

// ─── ROOM_LIST ───────────────────────────────────────────────────────────────

async fn handle_room_list(_frame: Frame, session: &mut SessionState, state: &SharedState) -> Result<()> {
    let rooms = room::Entity::find()
        .filter(room::Column::DeletedAt.is_null())
        .all(&state.db).await?;

    let room_map = state.room_members.read().await;
    let resp = RoomListResp {
        rooms: rooms.into_iter().map(|r| {
            let count = room_map.get(&r.id).map(|m| m.len() as u32).unwrap_or(0);
            neomsn_shared::proto::payload::RoomInfo { room_id: r.id, name: r.name, member_count: count }
        }).collect(),
    };

    session.send(Frame::new(Opcode::RoomListResp, resp.encode())).await;
    Ok(())
}

// ─── ROOM_JOIN ───────────────────────────────────────────────────────────────

async fn handle_room_join(frame: Frame, session: &mut SessionState, state: &SharedState) -> Result<()> {
    let p = payload::RoomJoin::decode(&frame.payload).map_err(|_| anyhow::anyhow!("decode"))?;
    let user_id = session.user_id.unwrap();

    state.room_members.write().await
        .entry(p.room_id).or_default()
        .insert(user_id);

    // Persist membership.
    let exists = room_member::Entity::find()
        .filter(room_member::Column::RoomId.eq(p.room_id))
        .filter(room_member::Column::UserId.eq(user_id))
        .filter(room_member::Column::LeftAt.is_null())
        .one(&state.db).await?.is_some();

    if !exists {
        room_member::ActiveModel {
            room_id: Set(p.room_id),
            user_id: Set(user_id),
            role: Set("member".into()),
            joined_at: Set(Utc::now()),
            left_at: Set(None),
        }.insert(&state.db).await?;
    }
    Ok(())
}

// ─── ROOM_LEAVE ──────────────────────────────────────────────────────────────

async fn handle_room_leave(frame: Frame, session: &mut SessionState, state: &SharedState) -> Result<()> {
    let p = payload::RoomLeave::decode(&frame.payload).map_err(|_| anyhow::anyhow!("decode"))?;
    let user_id = session.user_id.unwrap();

    state.room_members.write().await
        .entry(p.room_id).or_default()
        .remove(&user_id);

    if let Some(m) = room_member::Entity::find()
        .filter(room_member::Column::RoomId.eq(p.room_id))
        .filter(room_member::Column::UserId.eq(user_id))
        .filter(room_member::Column::LeftAt.is_null())
        .one(&state.db).await? {
        let mut am: room_member::ActiveModel = m.into();
        am.left_at = Set(Some(Utc::now()));
        am.update(&state.db).await?;
    }
    Ok(())
}

// ─── PRESENCE_SET ────────────────────────────────────────────────────────────

async fn handle_presence_set(frame: Frame, session: &mut SessionState, state: &SharedState) -> Result<()> {
    let p = payload::PresenceSet::decode(&frame.payload).map_err(|_| anyhow::anyhow!("decode"))?;
    let user_id = session.user_id.unwrap();

    state.presence.write().await.insert(user_id, p.status);

    // Notify accepted contacts.
    let contacts = contact::Entity::find()
        .filter(contact::Column::OwnerId.eq(user_id))
        .filter(contact::Column::State.eq("accepted"))
        .all(&state.db).await?;

    let update_frame = Frame::new(
        Opcode::PresenceUpdate,
        PresenceUpdate { user_id, status: p.status }.encode(),
    );
    for c in contacts {
        state.send_to_user(c.contact_id, update_frame.clone()).await;
    }
    Ok(())
}

// ─── PROFILE_GET ─────────────────────────────────────────────────────────────

async fn handle_profile_get(frame: Frame, session: &mut SessionState, state: &SharedState) -> Result<()> {
    let p = payload::ContactUserId::decode(&frame.payload).map_err(|_| anyhow::anyhow!("decode"))?;

    if let Some(u) = user::Entity::find_by_id(p.user_id).one(&state.db).await? {
        session.send(Frame::new(
            Opcode::ProfileResp,
            ProfileResp {
                user_id: u.id,
                username: u.username,
                display_name: u.display_name,
                personal_message: u.personal_message,
                avatar_url: u.avatar_url,
            }.encode(),
        )).await;
    }
    Ok(())
}

// ─── PROFILE_UPDATE ──────────────────────────────────────────────────────────

async fn handle_profile_update(frame: Frame, session: &mut SessionState, state: &SharedState) -> Result<()> {
    let p = payload::ProfileUpdate::decode(&frame.payload).map_err(|_| anyhow::anyhow!("decode"))?;
    let user_id = session.user_id.unwrap();

    if let Some(u) = user::Entity::find_by_id(user_id).one(&state.db).await? {
        let mut am: user::ActiveModel = u.into();
        am.display_name = Set(p.display_name);
        am.personal_message = Set(p.personal_message);
        am.update(&state.db).await?;
    }

    session.send(Frame::new(Opcode::ProfileUpdateOk, vec![])).await;
    Ok(())
}

// ─── helpers ─────────────────────────────────────────────────────────────────

async fn broadcast_to_context(
    ctx_type: &ContextType,
    context_id: Uuid,
    sender: Uuid,
    frame: Frame,
    state: &SharedState,
) {
    match ctx_type {
        ContextType::Room => {
            state.broadcast_room(context_id, frame, Some(sender)).await;
        }
        ContextType::Dm => {
            // context_id is the DirectConversation.id; look up participants.
            if let Ok(Some(conv)) = direct_conversation::Entity::find_by_id(context_id)
                .one(&state.db).await {
                state.broadcast_dm(conv.user_a, conv.user_b, frame, Some(sender)).await;
            }
        }
    }
}
