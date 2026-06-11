//! Typed payload structs for each NMP opcode.
//!
//! Each struct implements `encode() -> Vec<u8>` and `decode(&[u8]) -> Result<Self>`.
//! Binary layout uses big-endian integers. Strings are length-prefixed (u32 + utf8).
//! UUIDs are 16 raw bytes.

use uuid::Uuid;

// ─── low-level helpers ───────────────────────────────────────────────────────

pub struct Writer(Vec<u8>);

impl Writer {
    pub fn new() -> Self { Self(Vec::new()) }
    pub fn u8(&mut self, v: u8) { self.0.push(v); }
    pub fn u16(&mut self, v: u16) { self.0.extend_from_slice(&v.to_be_bytes()); }
    pub fn u32(&mut self, v: u32) { self.0.extend_from_slice(&v.to_be_bytes()); }
    pub fn u64(&mut self, v: u64) { self.0.extend_from_slice(&v.to_be_bytes()); }
    pub fn uuid(&mut self, v: &Uuid) { self.0.extend_from_slice(v.as_bytes()); }
    pub fn string(&mut self, s: &str) {
        let b = s.as_bytes();
        self.u32(b.len() as u32);
        self.0.extend_from_slice(b);
    }
    pub fn finish(self) -> Vec<u8> { self.0 }
}

pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

#[derive(Debug)]
pub struct DecodeError;

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "payload decode error")
    }
}
impl std::error::Error for DecodeError {}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self { Self { buf, pos: 0 } }

    fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        if self.pos + n > self.buf.len() { return Err(DecodeError); }
        let s = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    pub fn u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.take(1)?[0])
    }
    pub fn u16(&mut self) -> Result<u16, DecodeError> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }
    pub fn u32(&mut self) -> Result<u32, DecodeError> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
    pub fn u64(&mut self) -> Result<u64, DecodeError> {
        let b = self.take(8)?;
        Ok(u64::from_be_bytes(b.try_into().unwrap()))
    }
    pub fn uuid(&mut self) -> Result<Uuid, DecodeError> {
        let b = self.take(16)?;
        Ok(Uuid::from_bytes(b.try_into().unwrap()))
    }
    pub fn string(&mut self) -> Result<String, DecodeError> {
        let len = self.u32()? as usize;
        let b = self.take(len)?;
        String::from_utf8(b.to_vec()).map_err(|_| DecodeError)
    }
}

// ─── Presence status (shared enum) ───────────────────────────────────────────

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceStatus {
    Online    = 0,
    Away      = 1,
    Busy      = 2,
    Invisible = 3,
    Offline   = 4,
}

impl TryFrom<u8> for PresenceStatus {
    type Error = DecodeError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Online),
            1 => Ok(Self::Away),
            2 => Ok(Self::Busy),
            3 => Ok(Self::Invisible),
            4 => Ok(Self::Offline),
            _ => Err(DecodeError),
        }
    }
}

// ─── Context (room or DM) ─────────────────────────────────────────────────────

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextType { Room = 0, Dm = 1 }

impl TryFrom<u8> for ContextType {
    type Error = DecodeError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v { 0 => Ok(Self::Room), 1 => Ok(Self::Dm), _ => Err(DecodeError) }
    }
}

// ─── 0x01  HELLO ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Hello {
    pub proto_version: u16,
    pub device_id: Uuid,
}

impl Hello {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.u16(self.proto_version);
        w.uuid(&self.device_id);
        w.finish()
    }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(buf);
        Ok(Self { proto_version: r.u16()?, device_id: r.uuid()? })
    }
}

// ─── 0x02  AUTH ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Auth { pub token: String }

impl Auth {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new(); w.string(&self.token); w.finish()
    }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(buf); Ok(Self { token: r.string()? })
    }
}

// ─── 0x03  AUTH_OK ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AuthOk {
    pub user_id: Uuid,
    pub display_name: String,
    pub personal_message: String,
}

impl AuthOk {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.uuid(&self.user_id);
        w.string(&self.display_name);
        w.string(&self.personal_message);
        w.finish()
    }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(buf);
        Ok(Self { user_id: r.uuid()?, display_name: r.string()?, personal_message: r.string()? })
    }
}

// ─── 0x04  AUTH_FAIL ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AuthFail { pub reason: String }

impl AuthFail {
    pub fn encode(&self) -> Vec<u8> { let mut w = Writer::new(); w.string(&self.reason); w.finish() }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(buf); Ok(Self { reason: r.string()? })
    }
}

// ─── 0x10  MSG_CHUNK ─────────────────────────────────────────────────────────
// An edit to the streaming message: truncate the accumulated text to
// `truncate_to` bytes (the common prefix with the previous state), then append
// `delta`. Pure typing is truncate_to == current length; backspace is a
// shorter truncate_to with an empty delta.

#[derive(Debug, Clone)]
pub struct MsgChunk {
    pub msg_id: Uuid,
    pub context_type: ContextType,
    pub context_id: Uuid,
    pub author_id: Uuid,
    pub truncate_to: u32,
    pub delta: String,
}

impl MsgChunk {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.uuid(&self.msg_id);
        w.u8(self.context_type as u8);
        w.uuid(&self.context_id);
        w.uuid(&self.author_id);
        w.u32(self.truncate_to);
        w.string(&self.delta);
        w.finish()
    }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(buf);
        Ok(Self {
            msg_id: r.uuid()?,
            context_type: ContextType::try_from(r.u8()?)?,
            context_id: r.uuid()?,
            author_id: r.uuid()?,
            truncate_to: r.u32()?,
            delta: r.string()?,
        })
    }
}

// ─── 0x11  MSG_COMPLETE ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MsgComplete {
    pub msg_id: Uuid,
    pub context_type: ContextType,
    pub context_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
}

impl MsgComplete {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.uuid(&self.msg_id);
        w.u8(self.context_type as u8);
        w.uuid(&self.context_id);
        w.uuid(&self.author_id);
        w.string(&self.content);
        w.finish()
    }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(buf);
        Ok(Self {
            msg_id: r.uuid()?,
            context_type: ContextType::try_from(r.u8()?)?,
            context_id: r.uuid()?,
            author_id: r.uuid()?,
            content: r.string()?,
        })
    }
}

// ─── 0x12  MSG_DELETE ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MsgDelete {
    pub msg_id: Uuid,
    pub context_type: ContextType,
    pub context_id: Uuid,
}

impl MsgDelete {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.uuid(&self.msg_id); w.u8(self.context_type as u8); w.uuid(&self.context_id);
        w.finish()
    }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(buf);
        Ok(Self { msg_id: r.uuid()?, context_type: ContextType::try_from(r.u8()?)?, context_id: r.uuid()? })
    }
}

// ─── 0x13  NUDGE ─────────────────────────────────────────────────────────────
// "Chamar atenção": shakes the conversation window of everyone else in the
// context and plays a buzz. Ephemeral — relayed, never persisted.

#[derive(Debug, Clone)]
pub struct Nudge {
    pub context_type: ContextType,
    pub context_id: Uuid,
    pub author_id: Uuid,
}

impl Nudge {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.u8(self.context_type as u8);
        w.uuid(&self.context_id);
        w.uuid(&self.author_id);
        w.finish()
    }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(buf);
        Ok(Self {
            context_type: ContextType::try_from(r.u8()?)?,
            context_id: r.uuid()?,
            author_id: r.uuid()?,
        })
    }
}

// ─── 0x24  ROOM_JOIN ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RoomJoin { pub room_id: Uuid }
impl RoomJoin {
    pub fn encode(&self) -> Vec<u8> { let mut w = Writer::new(); w.uuid(&self.room_id); w.finish() }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        Ok(Self { room_id: Reader::new(buf).uuid()? })
    }
}

// ─── 0x25  ROOM_LEAVE ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RoomLeave { pub room_id: Uuid }
impl RoomLeave {
    pub fn encode(&self) -> Vec<u8> { let mut w = Writer::new(); w.uuid(&self.room_id); w.finish() }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        Ok(Self { room_id: Reader::new(buf).uuid()? })
    }
}

// ─── 0x27  ROOM_LIST_RESP ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RoomInfo { pub room_id: Uuid, pub name: String, pub member_count: u32 }

#[derive(Debug, Clone)]
pub struct RoomListResp { pub rooms: Vec<RoomInfo> }

impl RoomListResp {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.u32(self.rooms.len() as u32);
        for r in &self.rooms { w.uuid(&r.room_id); w.string(&r.name); w.u32(r.member_count); }
        w.finish()
    }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(buf);
        let count = r.u32()? as usize;
        let mut rooms = Vec::with_capacity(count);
        for _ in 0..count {
            rooms.push(RoomInfo { room_id: r.uuid()?, name: r.string()?, member_count: r.u32()? });
        }
        Ok(Self { rooms })
    }
}

// ─── 0x2A  CHAT_INVITE ───────────────────────────────────────────────────────
// Invite a user into the current conversation (MSN style). If the context is a
// DM, the server upgrades it to an ephemeral room with all three participants.

#[derive(Debug, Clone)]
pub struct ChatInvite {
    pub context_type: ContextType,
    pub context_id: Uuid,
    pub user_id: Uuid,
}

impl ChatInvite {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.u8(self.context_type as u8);
        w.uuid(&self.context_id);
        w.uuid(&self.user_id);
        w.finish()
    }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(buf);
        Ok(Self {
            context_type: ContextType::try_from(r.u8()?)?,
            context_id: r.uuid()?,
            user_id: r.uuid()?,
        })
    }
}

// ─── 0x2B  CHAT_JOINED ───────────────────────────────────────────────────────
// Sent to every participant of a group conversation when it is created or when
// they are pulled into it. `origin_context_id` is the DM conversation the room
// was upgraded from (nil UUID when not applicable) so existing windows can be
// converted in place.

#[derive(Debug, Clone)]
pub struct MemberInfo {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
}

#[derive(Debug, Clone)]
pub struct ChatJoined {
    pub room_id: Uuid,
    pub origin_context_id: Uuid,
    pub inviter_name: String,
    pub members: Vec<MemberInfo>,
}

impl ChatJoined {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.uuid(&self.room_id);
        w.uuid(&self.origin_context_id);
        w.string(&self.inviter_name);
        w.u32(self.members.len() as u32);
        for m in &self.members {
            w.uuid(&m.user_id);
            w.string(&m.username);
            w.string(&m.display_name);
        }
        w.finish()
    }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(buf);
        let room_id = r.uuid()?;
        let origin_context_id = r.uuid()?;
        let inviter_name = r.string()?;
        let count = r.u32()? as usize;
        let mut members = Vec::with_capacity(count);
        for _ in 0..count {
            members.push(MemberInfo {
                user_id: r.uuid()?,
                username: r.string()?,
                display_name: r.string()?,
            });
        }
        Ok(Self { room_id, origin_context_id, inviter_name, members })
    }
}

// ─── 0x29  ROOM_EVENT ────────────────────────────────────────────────────────

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomEventKind { Joined = 0, Left = 1 }

impl TryFrom<u8> for RoomEventKind {
    type Error = DecodeError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v { 0 => Ok(Self::Joined), 1 => Ok(Self::Left), _ => Err(DecodeError) }
    }
}

#[derive(Debug, Clone)]
pub struct RoomEvent {
    pub room_id: Uuid,
    pub kind: RoomEventKind,
    pub user_id: Uuid,
    pub display_name: String,
}

impl RoomEvent {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.uuid(&self.room_id);
        w.u8(self.kind as u8);
        w.uuid(&self.user_id);
        w.string(&self.display_name);
        w.finish()
    }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(buf);
        Ok(Self {
            room_id: r.uuid()?,
            kind: RoomEventKind::try_from(r.u8()?)?,
            user_id: r.uuid()?,
            display_name: r.string()?,
        })
    }
}

// ─── 0x30  DM_OPEN ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DmOpen { pub username: String }
impl DmOpen {
    pub fn encode(&self) -> Vec<u8> { let mut w = Writer::new(); w.string(&self.username); w.finish() }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        Ok(Self { username: Reader::new(buf).string()? })
    }
}

// ─── 0x31  DM_OPEN_RESP ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DmOpenResp {
    pub conversation_id: Uuid,
    pub user_id: Uuid,
    pub display_name: String,
}
impl DmOpenResp {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.uuid(&self.conversation_id); w.uuid(&self.user_id); w.string(&self.display_name);
        w.finish()
    }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(buf);
        Ok(Self { conversation_id: r.uuid()?, user_id: r.uuid()?, display_name: r.string()? })
    }
}

// ─── 0x40  PRESENCE_SET ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PresenceSet { pub status: PresenceStatus }
impl PresenceSet {
    pub fn encode(&self) -> Vec<u8> { vec![self.status as u8] }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.is_empty() { return Err(DecodeError); }
        Ok(Self { status: PresenceStatus::try_from(buf[0])? })
    }
}

// ─── 0x41  PRESENCE_UPDATE ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PresenceUpdate { pub user_id: Uuid, pub status: PresenceStatus }
impl PresenceUpdate {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new(); w.uuid(&self.user_id); w.u8(self.status as u8); w.finish()
    }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(buf);
        Ok(Self { user_id: r.uuid()?, status: PresenceStatus::try_from(r.u8()?)? })
    }
}

// ─── 0x50  SYNC_REQUEST ──────────────────────────────────────────────────────
// Ask for the most recent completed messages of a context (history load).

#[derive(Debug, Clone)]
pub struct SyncRequest {
    pub context_type: ContextType,
    pub context_id: Uuid,
    pub limit: u32,
}

impl SyncRequest {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.u8(self.context_type as u8);
        w.uuid(&self.context_id);
        w.u32(self.limit);
        w.finish()
    }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(buf);
        Ok(Self {
            context_type: ContextType::try_from(r.u8()?)?,
            context_id: r.uuid()?,
            limit: r.u32()?,
        })
    }
}

// ─── 0x51  SYNC_RESPONSE ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct HistoryMessage {
    pub msg_id: Uuid,
    pub author_id: Uuid,
    pub author_name: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct SyncResponse {
    pub context_type: ContextType,
    pub context_id: Uuid,
    pub messages: Vec<HistoryMessage>,
}

impl SyncResponse {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.u8(self.context_type as u8);
        w.uuid(&self.context_id);
        w.u32(self.messages.len() as u32);
        for m in &self.messages {
            w.uuid(&m.msg_id);
            w.uuid(&m.author_id);
            w.string(&m.author_name);
            w.string(&m.content);
        }
        w.finish()
    }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(buf);
        let context_type = ContextType::try_from(r.u8()?)?;
        let context_id = r.uuid()?;
        let count = r.u32()? as usize;
        let mut messages = Vec::with_capacity(count);
        for _ in 0..count {
            messages.push(HistoryMessage {
                msg_id: r.uuid()?,
                author_id: r.uuid()?,
                author_name: r.string()?,
                content: r.string()?,
            });
        }
        Ok(Self { context_type, context_id, messages })
    }
}

// ─── 0x61  PROFILE_RESP ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ProfileResp {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub personal_message: String,
    pub avatar_url: String,
}
impl ProfileResp {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.uuid(&self.user_id);
        w.string(&self.username);
        w.string(&self.display_name);
        w.string(&self.personal_message);
        w.string(&self.avatar_url);
        w.finish()
    }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(buf);
        Ok(Self {
            user_id: r.uuid()?,
            username: r.string()?,
            display_name: r.string()?,
            personal_message: r.string()?,
            avatar_url: r.string()?,
        })
    }
}

// ─── 0x62  PROFILE_UPDATE ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ProfileUpdate { pub display_name: String, pub personal_message: String }
impl ProfileUpdate {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new(); w.string(&self.display_name); w.string(&self.personal_message); w.finish()
    }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(buf);
        Ok(Self { display_name: r.string()?, personal_message: r.string()? })
    }
}

// ─── 0x71  CONTACT_LIST_RESP ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ContactEntry {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub presence: PresenceStatus,
}

#[derive(Debug, Clone)]
pub struct ContactListResp { pub contacts: Vec<ContactEntry> }

impl ContactListResp {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.u32(self.contacts.len() as u32);
        for c in &self.contacts {
            w.uuid(&c.user_id);
            w.string(&c.username);
            w.string(&c.display_name);
            w.u8(c.presence as u8);
        }
        w.finish()
    }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(buf);
        let count = r.u32()? as usize;
        let mut contacts = Vec::with_capacity(count);
        for _ in 0..count {
            contacts.push(ContactEntry {
                user_id: r.uuid()?,
                username: r.string()?,
                display_name: r.string()?,
                presence: PresenceStatus::try_from(r.u8()?)?,
            });
        }
        Ok(Self { contacts })
    }
}

// ─── 0x72  CONTACT_ADD ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ContactAdd { pub username: String }
impl ContactAdd {
    pub fn encode(&self) -> Vec<u8> { let mut w = Writer::new(); w.string(&self.username); w.finish() }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        Ok(Self { username: Reader::new(buf).string()? })
    }
}

// ─── 0x73  CONTACT_ADD_OK ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ContactAddOk { pub user_id: Uuid, pub username: String, pub display_name: String }
impl ContactAddOk {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.uuid(&self.user_id); w.string(&self.username); w.string(&self.display_name);
        w.finish()
    }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(buf);
        Ok(Self { user_id: r.uuid()?, username: r.string()?, display_name: r.string()? })
    }
}

// ─── 0x74–0x76  CONTACT_REMOVE / BLOCK / UNBLOCK ─────────────────────────────

#[derive(Debug, Clone)]
pub struct ContactUserId { pub user_id: Uuid }
impl ContactUserId {
    pub fn encode(&self) -> Vec<u8> { let mut w = Writer::new(); w.uuid(&self.user_id); w.finish() }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        Ok(Self { user_id: Reader::new(buf).uuid()? })
    }
}

// ─── 0x77  CONTACT_REQUEST ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ContactRequest { pub user_id: Uuid, pub username: String, pub display_name: String }
impl ContactRequest {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.uuid(&self.user_id); w.string(&self.username); w.string(&self.display_name);
        w.finish()
    }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(buf);
        Ok(Self { user_id: r.uuid()?, username: r.string()?, display_name: r.string()? })
    }
}

// ─── 0x78  CONTACT_ACCEPT ────────────────────────────────────────────────────
// Reuses ContactUserId (user_id = the requester being accepted).

// ─── 0x79  CONTACT_ACCEPT_OK ─────────────────────────────────────────────────
// Sent to both sides when a request is accepted.
// Carries the newly added contact's full info so the recipient can update
// their contact list without a separate CONTACT_LIST request.

#[derive(Debug, Clone)]
pub struct ContactAcceptOk {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub presence: PresenceStatus,
}

impl ContactAcceptOk {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.uuid(&self.user_id);
        w.string(&self.username);
        w.string(&self.display_name);
        w.u8(self.presence as u8);
        w.finish()
    }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(buf);
        Ok(Self {
            user_id: r.uuid()?,
            username: r.string()?,
            display_name: r.string()?,
            presence: PresenceStatus::try_from(r.u8()?)?,
        })
    }
}

// ─── 0x7A  CONTACT_REJECT ────────────────────────────────────────────────────
// Reuses ContactUserId (user_id = the requester being rejected).
// Server silently deletes the pending entry; requester is NOT notified.

// ─── 0xF0  ERROR ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Error { pub code: u16, pub message: String }
impl Error {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new(); w.u16(self.code); w.string(&self.message); w.finish()
    }
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(buf); Ok(Self { code: r.u16()?, message: r.string()? })
    }
}
