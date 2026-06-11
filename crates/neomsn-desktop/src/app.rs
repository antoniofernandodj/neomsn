use std::{collections::HashMap, sync::Arc};
use iced::{window, Element, Size, Subscription, Task};
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;
use neomsn_shared::{
    domain::{
        contact::{Contact, ContactState},
        message::ContextType,
        user::PresenceStatus,
    },
    proto::{Frame, Opcode, payload},
    widgets::theme::MsnTheme,
};
use crate::{
    net::{
        http::{AuthResponse, HttpClient},
        nmp::{self, NmpClient, ServerEvent},
    },
    screens::{
        chat::{ChatMsg, ChatScreen},
        contact_list::{ContactListMsg, ContactListScreen, PendingRequest},
        login::{LoginMsg, LoginScreen},
    },
};

// ─── Top-level message ────────────────────────────────────────────────────────

type RxHandle = Arc<Mutex<mpsc::Receiver<ServerEvent>>>;

const HISTORY_LIMIT: u32 = 50;

#[derive(Debug, Clone)]
pub enum AppMsg {
    Login(LoginMsg),
    LoginResult(Result<AuthResponse, String>),
    NmpConnected(NmpClient, AuthResponse, RxHandle),
    NmpEvent(ServerEvent, RxHandle),
    NmpDisconnected,
    ContactList(ContactListMsg),
    Chat(window::Id, ChatMsg),
    WindowOpened(window::Id),
    WindowClosed(window::Id),
}

// ─── App state ────────────────────────────────────────────────────────────────

pub enum Phase {
    Login(LoginScreen),
    Main {
        contact_list: ContactListScreen,
        nmp: NmpClient,
        self_user_id: Uuid,
    },
}

pub struct App {
    phase: Phase,
    http: HttpClient,
    main_window: window::Id,
    /// One native window per open conversation.
    chats: HashMap<window::Id, ChatScreen>,
}

fn main_window_settings() -> window::Settings {
    window::Settings {
        size: Size::new(300.0, 560.0),
        min_size: Some(Size::new(260.0, 420.0)),
        ..Default::default()
    }
}

fn chat_window_settings() -> window::Settings {
    window::Settings {
        size: Size::new(560.0, 470.0),
        min_size: Some(Size::new(440.0, 380.0)),
        ..Default::default()
    }
}

fn to_proto(ct: ContextType) -> payload::ContextType {
    match ct {
        ContextType::Room => payload::ContextType::Room,
        ContextType::Dm => payload::ContextType::Dm,
    }
}

impl App {
    pub fn new() -> (Self, Task<AppMsg>) {
        let (main_window, open) = window::open(main_window_settings());
        (
            Self {
                phase: Phase::Login(LoginScreen::new()),
                http: HttpClient::new(),
                main_window,
                chats: HashMap::new(),
            },
            open.map(AppMsg::WindowOpened),
        )
    }

    pub fn title(&self, window: window::Id) -> String {
        if let Some(chat) = self.chats.get(&window) {
            format!("{} — Conversa", chat.title())
        } else {
            "NeoMSN".into()
        }
    }

    pub fn theme(&self, _window: window::Id) -> iced::Theme {
        MsnTheme::iced()
    }

    pub fn subscription(&self) -> Subscription<AppMsg> {
        window::close_events().map(AppMsg::WindowClosed)
    }

    pub fn view(&self, window: window::Id) -> Element<'_, AppMsg> {
        if window == self.main_window {
            return match &self.phase {
                Phase::Login(ls) => ls.view().map(AppMsg::Login),
                Phase::Main { contact_list, .. } => contact_list.view().map(AppMsg::ContactList),
            };
        }
        if let Some(chat) = self.chats.get(&window) {
            let contacts: &[Contact] = match &self.phase {
                Phase::Main { contact_list, .. } => &contact_list.contacts,
                _ => &[],
            };
            return chat.view(contacts).map(move |m| AppMsg::Chat(window, m));
        }
        iced::widget::Space::new(0, 0).into()
    }

    pub fn update(&mut self, msg: AppMsg) -> Task<AppMsg> {
        match msg {
            AppMsg::Login(lm)              => self.handle_login_msg(lm),
            AppMsg::LoginResult(r)         => self.handle_login_result(r),
            AppMsg::NmpConnected(c, a, rx) => self.handle_nmp_connected(c, a, rx),
            AppMsg::NmpEvent(e, rx)        => self.handle_server_event(e, rx),
            AppMsg::NmpDisconnected        => self.handle_disconnected(),
            AppMsg::ContactList(clm)       => self.handle_contact_list_msg(clm),
            AppMsg::Chat(id, cm)           => self.handle_chat_msg(id, cm),
            AppMsg::WindowOpened(_)        => Task::none(),
            AppMsg::WindowClosed(id)       => self.handle_window_closed(id),
        }
    }

    // ── Window lifecycle ─────────────────────────────────────────────────────

    fn handle_window_closed(&mut self, id: window::Id) -> Task<AppMsg> {
        if id == self.main_window {
            return iced::exit();
        }
        if let Some(chat) = self.chats.remove(&id) {
            // Group conversations are live sessions: closing the window leaves.
            if chat.context_type == ContextType::Room
                && let Phase::Main { nmp, .. } = &self.phase
            {
                nmp.send(Frame::new(
                    Opcode::RoomLeave,
                    payload::RoomLeave { room_id: chat.context_id }.encode(),
                ));
            }
        }
        Task::none()
    }

    fn handle_disconnected(&mut self) -> Task<AppMsg> {
        self.phase = Phase::Login(LoginScreen::new());
        let close_all: Vec<Task<AppMsg>> = self.chats.drain()
            .map(|(id, _)| window::close(id))
            .collect();
        Task::batch(close_all)
    }

    /// Open a native window for a conversation and request its history.
    fn open_chat_window(&mut self, chat: ChatScreen, nmp: &NmpClient) -> Task<AppMsg> {
        nmp.send(Frame::new(
            Opcode::SyncRequest,
            payload::SyncRequest {
                context_type: to_proto(chat.context_type),
                context_id: chat.context_id,
                limit: HISTORY_LIMIT,
            }.encode(),
        ));
        let (id, task) = window::open(chat_window_settings());
        self.chats.insert(id, chat);
        task.map(AppMsg::WindowOpened)
    }

    fn chat_by_context(&mut self, context_id: Uuid) -> Option<&mut ChatScreen> {
        self.chats.values_mut().find(|c| c.context_id == context_id)
    }

    fn contact_list_mut(&mut self) -> Option<&mut ContactListScreen> {
        match &mut self.phase {
            Phase::Main { contact_list, .. } => Some(contact_list),
            _ => None,
        }
    }

    fn self_display_name(&self) -> String {
        match &self.phase {
            Phase::Main { contact_list, .. } => contact_list.display_name.clone(),
            _ => String::new(),
        }
    }

    fn contact_display_name(&self, user_id: Uuid) -> Option<String> {
        match &self.phase {
            Phase::Main { contact_list, .. } => contact_list.contacts.iter()
                .find(|c| c.user_id == user_id)
                .map(|c| c.display_name.clone()),
            _ => None,
        }
    }

    // ── Login ────────────────────────────────────────────────────────────────

    fn handle_login_msg(&mut self, msg: LoginMsg) -> Task<AppMsg> {
        if let Phase::Login(ref mut ls) = self.phase {
            match msg {
                LoginMsg::UsernameChanged(v)    => ls.username = v,
                LoginMsg::PasswordChanged(v)    => ls.password = v,
                LoginMsg::DisplayNameChanged(v) => ls.display_name = v,
                LoginMsg::ToggleSignup          => { ls.signup_mode = !ls.signup_mode; ls.error = None; }
                LoginMsg::Submit => {
                    ls.loading = true;
                    ls.error = None;
                    let http = self.http.clone();
                    let username = ls.username.clone();
                    let password = ls.password.clone();
                    let display_name = ls.display_name.clone();
                    let signup = ls.signup_mode;
                    return Task::perform(
                        async move {
                            if signup {
                                http.signup(&username, &password, &display_name).await
                            } else {
                                http.login(&username, &password).await
                            }
                        },
                        |r| AppMsg::LoginResult(r.map_err(|e| e.to_string())),
                    );
                }
            }
        }
        Task::none()
    }

    fn handle_login_result(&mut self, result: Result<AuthResponse, String>) -> Task<AppMsg> {
        match result {
            Err(e) => {
                if let Phase::Login(ref mut ls) = self.phase {
                    ls.error = Some(e);
                    ls.loading = false;
                }
                Task::none()
            }
            Ok(auth) => {
                let token = auth.token.clone();
                let auth2 = auth.clone();
                let device_id = match Uuid::parse_str(&auth.device_id) {
                    Ok(id) => id,
                    Err(e) => {
                        if let Phase::Login(ref mut ls) = self.phase {
                            ls.error = Some(e.to_string());
                            ls.loading = false;
                        }
                        return Task::none();
                    }
                };
                Task::perform(
                    async move { nmp::connect(token, device_id).await },
                    move |res| match res {
                        Ok((client, rx)) => {
                            let rx = Arc::new(Mutex::new(rx));
                            AppMsg::NmpConnected(client, auth2.clone(), rx)
                        }
                        Err(e) => AppMsg::LoginResult(Err(e.to_string())),
                    },
                )
            }
        }
    }

    fn handle_nmp_connected(&mut self, client: NmpClient, auth: AuthResponse, rx: RxHandle) -> Task<AppMsg> {
        client.send(Frame::new(Opcode::ContactList, vec![]));

        let uid = Uuid::parse_str(&auth.user_id).unwrap_or(Uuid::nil());
        self.phase = Phase::Main {
            contact_list: ContactListScreen::new("Carregando…".into()),
            nmp: client,
            self_user_id: uid,
        };

        // Start the event receive chain.
        wait_for_next_event(rx)
    }

    // ── Server events ────────────────────────────────────────────────────────

    fn handle_server_event(&mut self, event: ServerEvent, rx: RxHandle) -> Task<AppMsg> {
        // Always schedule receiving the next event.
        let next = wait_for_next_event(rx);

        let (nmp, self_id) = match &self.phase {
            Phase::Main { nmp, self_user_id, .. } => (nmp.clone(), *self_user_id),
            _ => return next,
        };

        match event {
            ServerEvent::AuthOk(p) => {
                if let Phase::Main { contact_list, self_user_id, .. } = &mut self.phase {
                    *self_user_id = p.user_id;
                    contact_list.display_name = p.display_name;
                    contact_list.personal_message = p.personal_message;
                }
            }
            ServerEvent::ContactListResp(p) => {
                if let Some(contact_list) = self.contact_list_mut() {
                    contact_list.contacts = p.contacts.into_iter().map(|e| Contact {
                        user_id: e.user_id,
                        username: e.username,
                        display_name: e.display_name,
                        presence: PresenceStatus::from(e.presence),
                        state: ContactState::Accepted,
                    }).collect();
                }
            }
            ServerEvent::ContactRequest(p) => {
                if let Some(contact_list) = self.contact_list_mut() {
                    let already = contact_list.pending_requests.iter().any(|r| r.user_id == p.user_id);
                    if !already {
                        contact_list.pending_requests.push(PendingRequest {
                            user_id: p.user_id,
                            username: p.username,
                            display_name: p.display_name,
                        });
                    }
                }
            }
            ServerEvent::ContactAcceptOk(p) => {
                if let Some(contact_list) = self.contact_list_mut() {
                    contact_list.pending_requests.retain(|r| r.user_id != p.user_id);
                    let already = contact_list.contacts.iter().any(|c| c.user_id == p.user_id);
                    if !already {
                        contact_list.contacts.push(Contact {
                            user_id: p.user_id,
                            username: p.username,
                            display_name: p.display_name,
                            presence: PresenceStatus::from(p.presence),
                            state: ContactState::Accepted,
                        });
                    }
                }
            }
            ServerEvent::PresenceUpdate(p) => {
                if let Some(contact_list) = self.contact_list_mut() {
                    for c in &mut contact_list.contacts {
                        if c.user_id == p.user_id {
                            c.presence = PresenceStatus::from(p.status);
                        }
                    }
                }
            }
            ServerEvent::DmOpenResp(p) => {
                if let Some((id, _)) = self.chats.iter().find(|(_, c)| c.context_id == p.conversation_id) {
                    return Task::batch([next, window::gain_focus(*id)]);
                }
                let chat = ChatScreen::new_dm(
                    p.conversation_id,
                    p.user_id,
                    p.display_name,
                    self_id,
                    self.self_display_name(),
                );
                let open = self.open_chat_window(chat, &nmp);
                return Task::batch([next, open]);
            }
            ServerEvent::MsgChunk(p) => {
                let author_name = self.contact_display_name(p.author_id)
                    .unwrap_or_else(|| "Contato".into());
                let self_name = self.self_display_name();

                if let Some(chat) = self.chat_by_context(p.context_id) {
                    chat.apply_chunk(p.msg_id, p.author_id, &author_name, p.truncate_to as usize, &p.delta);
                } else if p.context_type == payload::ContextType::Dm {
                    // Incoming message on a closed conversation: pop a window,
                    // exactly like classic MSN.
                    let mut chat = ChatScreen::new_dm(
                        p.context_id,
                        p.author_id,
                        author_name.clone(),
                        self_id,
                        self_name,
                    );
                    chat.apply_chunk(p.msg_id, p.author_id, &author_name, p.truncate_to as usize, &p.delta);
                    let open = self.open_chat_window(chat, &nmp);
                    return Task::batch([next, open]);
                }
            }
            ServerEvent::MsgComplete(p) => {
                if let Some(chat) = self.chat_by_context(p.context_id) {
                    chat.complete_message(p.msg_id, &p.content);
                }
            }
            ServerEvent::MsgDelete(p) => {
                if let Some(chat) = self.chat_by_context(p.context_id) {
                    chat.delete_message(p.msg_id);
                }
            }
            ServerEvent::SyncResponse(p) => {
                if let Some(chat) = self.chat_by_context(p.context_id) {
                    chat.prepend_history(
                        p.messages.into_iter()
                            .map(|m| (m.msg_id, m.author_id, m.author_name, m.content))
                            .collect(),
                    );
                }
            }
            ServerEvent::ChatJoined(p) => {
                let members: Vec<(Uuid, String)> = p.members.iter()
                    .map(|m| (m.user_id, m.display_name.clone()))
                    .collect();

                if self.chat_by_context(p.room_id).is_some() {
                    // Already in this group conversation — nothing to do.
                } else if let Some(chat) = self.chat_by_context(p.origin_context_id) {
                    // Our DM just became a group conversation: convert in place.
                    let old_ids: Vec<Uuid> = chat.members.iter().map(|(id, _)| *id).collect();
                    chat.upgrade_to_room(p.room_id, members.clone());
                    for (id, name) in &members {
                        if !old_ids.contains(id) {
                            chat.push_system(format!("{name} entrou na conversa."));
                        }
                    }
                } else {
                    // We were invited: open a window for the group conversation.
                    let mut chat = ChatScreen::new_room(p.room_id, members, self_id);
                    chat.push_system(format!("{} convidou você para esta conversa.", p.inviter_name));
                    let open = self.open_chat_window(chat, &nmp);
                    return Task::batch([next, open]);
                }
            }
            ServerEvent::RoomEvent(p) => {
                if let Some(chat) = self.chat_by_context(p.room_id) {
                    match p.kind {
                        payload::RoomEventKind::Joined => {
                            if chat.member_name(p.user_id).is_none() {
                                chat.members.push((p.user_id, p.display_name.clone()));
                            }
                            chat.push_system(format!("{} entrou na conversa.", p.display_name));
                        }
                        payload::RoomEventKind::Left => {
                            chat.members.retain(|(id, _)| *id != p.user_id);
                            chat.push_system(format!("{} saiu da conversa.", p.display_name));
                        }
                    }
                }
            }
            ServerEvent::Disconnected => {
                return Task::batch([Task::done(AppMsg::NmpDisconnected)]);
            }
            _ => {}
        }
        next
    }

    // ── Contact list interactions ────────────────────────────────────────────

    fn handle_contact_list_msg(&mut self, msg: ContactListMsg) -> Task<AppMsg> {
        let Phase::Main { contact_list, nmp, .. } = &mut self.phase else {
            return Task::none();
        };

        match msg {
            ContactListMsg::SearchChanged(v)   => contact_list.search = v,
            ContactListMsg::AddContactInput(v) => contact_list.add_input = v,
            ContactListMsg::AddContact => {
                if !contact_list.add_input.is_empty() {
                    nmp.send(Frame::new(
                        Opcode::ContactAdd,
                        payload::ContactAdd { username: contact_list.add_input.clone() }.encode(),
                    ));
                    contact_list.add_input.clear();
                }
            }
            ContactListMsg::OpenChat(contact_id) => {
                if let Some(c) = contact_list.contacts.iter().find(|c| c.user_id == contact_id) {
                    nmp.send(Frame::new(
                        Opcode::DmOpen,
                        payload::DmOpen { username: c.username.clone() }.encode(),
                    ));
                }
            }
            ContactListMsg::SetStatus(s) => {
                contact_list.status = s;
                nmp.send(Frame::new(
                    Opcode::PresenceSet,
                    payload::PresenceSet { status: s.into() }.encode(),
                ));
            }
            ContactListMsg::AcceptRequest(user_id) => {
                contact_list.pending_requests.retain(|r| r.user_id != user_id);
                nmp.send(Frame::new(
                    Opcode::ContactAccept,
                    payload::ContactUserId { user_id }.encode(),
                ));
            }
            ContactListMsg::RejectRequest(user_id) => {
                contact_list.pending_requests.retain(|r| r.user_id != user_id);
                nmp.send(Frame::new(
                    Opcode::ContactReject,
                    payload::ContactUserId { user_id }.encode(),
                ));
            }
        }
        Task::none()
    }

    // ── Chat window interactions ─────────────────────────────────────────────

    fn handle_chat_msg(&mut self, window_id: window::Id, msg: ChatMsg) -> Task<AppMsg> {
        let Phase::Main { contact_list, nmp, self_user_id } = &mut self.phase else {
            return Task::none();
        };
        let uid = *self_user_id;
        let self_display_name = contact_list.display_name.clone();
        let Some(chat) = self.chats.get_mut(&window_id) else {
            return Task::none();
        };

        match msg {
            ChatMsg::InputChanged(v) => {
                apply_input_change(chat, nmp, uid, &self_display_name, v);
            }
            ChatMsg::EmojiPicked(emoji) => {
                let v = format!("{}{}", chat.input, emoji);
                apply_input_change(chat, nmp, uid, &self_display_name, v);
            }
            ChatMsg::ToggleEmoji => {
                chat.emoji_open = !chat.emoji_open;
            }
            ChatMsg::Complete => {
                if !chat.input.is_empty() {
                    let msg_id = chat.current_msg_id;
                    let content = chat.input.clone();

                    nmp.send(Frame::new(
                        Opcode::MsgComplete,
                        payload::MsgComplete {
                            msg_id,
                            context_type: to_proto(chat.context_type),
                            context_id: chat.context_id,
                            author_id: uid,
                            content: content.clone(),
                        }.encode(),
                    ));

                    chat.complete_message(msg_id, &content);
                }
            }
            ChatMsg::ToggleInvite => {
                chat.invite_open = !chat.invite_open;
            }
            ChatMsg::Invite(user_id) => {
                nmp.send(Frame::new(
                    Opcode::ChatInvite,
                    payload::ChatInvite {
                        context_type: to_proto(chat.context_type),
                        context_id: chat.context_id,
                        user_id,
                    }.encode(),
                ));
                chat.invite_open = false;
            }
        }
        Task::none()
    }
}

// ─── Streaming input edits ────────────────────────────────────────────────────

/// Byte length of the common prefix of `a` and `b`, always at a char boundary.
fn common_prefix_len(a: &str, b: &str) -> usize {
    a.char_indices()
        .zip(b.chars())
        .find(|((_, ca), cb)| ca != cb)
        .map(|((i, _), _)| i)
        .unwrap_or_else(|| a.len().min(b.len()))
}

/// Propagate any input edit (typing, backspace, mid-text edits, emoji insert)
/// as a truncate+append chunk, so peers mirror deletions too. Erasing the whole
/// text cancels the streaming message with MSG_DELETE, like closing the field.
fn apply_input_change(
    chat: &mut ChatScreen,
    nmp: &NmpClient,
    author_id: Uuid,
    self_display_name: &str,
    v: String,
) {
    if v == chat.input {
        return;
    }
    let msg_id = chat.current_msg_id;

    if v.is_empty() {
        chat.input.clear();
        nmp.send(Frame::new(
            Opcode::MsgDelete,
            payload::MsgDelete {
                msg_id,
                context_type: to_proto(chat.context_type),
                context_id: chat.context_id,
            }.encode(),
        ));
        chat.delete_message(msg_id);
        chat.current_msg_id = Uuid::new_v4();
        return;
    }

    let prefix = common_prefix_len(&chat.input, &v);
    let delta = v[prefix..].to_string();
    chat.input = v;

    nmp.send(Frame::new(
        Opcode::MsgChunk,
        payload::MsgChunk {
            msg_id,
            context_type: to_proto(chat.context_type),
            context_id: chat.context_id,
            author_id,
            truncate_to: prefix as u32,
            delta: delta.clone(),
        }.encode(),
    ));

    chat.apply_chunk(msg_id, author_id, self_display_name, prefix, &delta);
}

// ─── Drive NMP receive channel as chained Tasks ───────────────────────────────

fn wait_for_next_event(rx: RxHandle) -> Task<AppMsg> {
    Task::perform(
        async move {
            let event = rx.lock().await.recv().await;
            (event, rx)
        },
        |(event, rx)| match event {
            Some(e) => AppMsg::NmpEvent(e, rx),
            None    => AppMsg::NmpDisconnected,
        },
    )
}
