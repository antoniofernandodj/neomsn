use std::sync::Arc;
use iced::{Element, Subscription, Task};
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;
use neomsn_shared::{
    domain::{
        contact::{Contact, ContactState},
        message::ContextType,
        user::PresenceStatus,
    },
    proto::{Frame, Opcode, payload},
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

#[derive(Debug, Clone)]
pub enum AppMsg {
    Login(LoginMsg),
    LoginResult(Result<AuthResponse, String>),
    NmpConnected(NmpClient, AuthResponse, RxHandle),
    NmpEvent(ServerEvent, RxHandle),
    NmpDisconnected,
    ContactList(ContactListMsg),
    Chat(usize, ChatMsg),
}

// ─── App state ────────────────────────────────────────────────────────────────

pub enum Screen {
    Login(LoginScreen),
    Main {
        contact_list: ContactListScreen,
        open_chats: Vec<ChatScreen>,
        active_chat: Option<usize>,
        nmp: NmpClient,
        self_user_id: Uuid,
        auth: AuthResponse,
    },
}

pub struct App {
    screen: Screen,
    http: HttpClient,
}

impl App {
    pub fn new() -> (Self, Task<AppMsg>) {
        (
            Self {
                screen: Screen::Login(LoginScreen::new()),
                http: HttpClient::new(),
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, msg: AppMsg) -> Task<AppMsg> {
        match msg {
            AppMsg::Login(lm)              => self.handle_login_msg(lm),
            AppMsg::LoginResult(r)         => self.handle_login_result(r),
            AppMsg::NmpConnected(c, a, rx) => self.handle_nmp_connected(c, a, rx),
            AppMsg::NmpEvent(e, rx)        => self.handle_server_event(e, rx),
            AppMsg::NmpDisconnected        => { self.screen = Screen::Login(LoginScreen::new()); Task::none() }
            AppMsg::ContactList(clm)       => self.handle_contact_list_msg(clm),
            AppMsg::Chat(idx, cm)          => self.handle_chat_msg(idx, cm),
        }
    }

    pub fn view(&self) -> Element<AppMsg> {
        match &self.screen {
            Screen::Login(ls) => ls.view().map(AppMsg::Login),
            Screen::Main { contact_list, open_chats, active_chat, .. } => {
                use iced::widget::row;
                let cl = contact_list.view().map(AppMsg::ContactList);
                if let Some(idx) = active_chat {
                    let i = *idx;
                    let chat = open_chats[i].view().map(move |m| AppMsg::Chat(i, m));
                    row![cl, chat].into()
                } else {
                    cl
                }
            }
        }
    }

    pub fn subscription(&self) -> Subscription<AppMsg> {
        Subscription::none()
    }

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn handle_login_msg(&mut self, msg: LoginMsg) -> Task<AppMsg> {
        if let Screen::Login(ref mut ls) = self.screen {
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
                if let Screen::Login(ref mut ls) = self.screen {
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
                        if let Screen::Login(ref mut ls) = self.screen {
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

        let cl = ContactListScreen::new("Carregando…".into());
        let uid = Uuid::parse_str(&auth.user_id).unwrap_or(Uuid::nil());

        self.screen = Screen::Main {
            contact_list: cl,
            open_chats: Vec::new(),
            active_chat: None,
            nmp: client,
            self_user_id: uid,
            auth,
        };

        // Start the event receive chain.
        wait_for_next_event(rx)
    }

    fn handle_server_event(&mut self, event: ServerEvent, rx: RxHandle) -> Task<AppMsg> {
        // Always schedule receiving the next event.
        let next = wait_for_next_event(rx);

        let Screen::Main { contact_list, open_chats, active_chat, self_user_id, .. } = &mut self.screen else {
            return next;
        };

        match event {
            ServerEvent::AuthOk(p) => {
                *self_user_id = p.user_id;
                contact_list.display_name = p.display_name;
                contact_list.personal_message = p.personal_message;
            }
            ServerEvent::ContactListResp(p) => {
                contact_list.contacts = p.contacts.into_iter().map(|e| Contact {
                    user_id: e.user_id,
                    username: e.username,
                    display_name: e.display_name,
                    presence: PresenceStatus::from(e.presence),
                    state: ContactState::Accepted,
                }).collect();
            }
            ServerEvent::ContactRequest(p) => {
                // Show in the pending requests panel if not already there.
                let already = contact_list.pending_requests.iter().any(|r| r.user_id == p.user_id);
                if !already {
                    contact_list.pending_requests.push(PendingRequest {
                        user_id: p.user_id,
                        username: p.username,
                        display_name: p.display_name,
                    });
                }
            }
            ServerEvent::ContactAcceptOk(p) => {
                // Remove from pending (in case it was our own request being accepted).
                contact_list.pending_requests.retain(|r| r.user_id != p.user_id);
                // Add to contact list if not already present.
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
            ServerEvent::PresenceUpdate(p) => {
                for c in &mut contact_list.contacts {
                    if c.user_id == p.user_id {
                        c.presence = PresenceStatus::from(p.status);
                    }
                }
            }
            ServerEvent::DmOpenResp(p) => {
                let uid = *self_user_id;
                let already = open_chats.iter().any(|c| c.context_id == p.conversation_id);
                if !already {
                    open_chats.push(ChatScreen::new(
                        p.conversation_id,
                        ContextType::Dm,
                        p.display_name,
                        uid,
                    ));
                }
                *active_chat = open_chats.iter().position(|c| c.context_id == p.conversation_id);
            }
            ServerEvent::MsgChunk(p) => {
                let display_name = contact_list.contacts.iter()
                    .find(|c| c.user_id == p.author_id)
                    .map(|c| c.display_name.clone())
                    .unwrap_or_else(|| p.author_id.to_string());
                for chat in open_chats.iter_mut() {
                    if chat.context_id == p.context_id {
                        chat.apply_chunk(p.msg_id, p.author_id, &display_name, &p.delta);
                    }
                }
            }
            ServerEvent::MsgComplete(p) => {
                for chat in open_chats.iter_mut() {
                    if chat.context_id == p.context_id {
                        chat.complete_message(p.msg_id, &p.content);
                    }
                }
            }
            ServerEvent::MsgDelete(p) => {
                for chat in open_chats.iter_mut() {
                    if chat.context_id == p.context_id {
                        chat.delete_message(p.msg_id);
                    }
                }
            }
            ServerEvent::Disconnected => {
                return Task::done(AppMsg::NmpDisconnected);
            }
            _ => {}
        }
        next
    }

    fn handle_contact_list_msg(&mut self, msg: ContactListMsg) -> Task<AppMsg> {
        let Screen::Main { contact_list, nmp, open_chats: _, active_chat: _, self_user_id: _, auth: _ } = &mut self.screen else {
            return Task::none();
        };

        match msg {
            ContactListMsg::SearchChanged(v)  => contact_list.search = v,
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

    fn handle_chat_msg(&mut self, idx: usize, msg: ChatMsg) -> Task<AppMsg> {
        let Screen::Main { open_chats, active_chat, nmp, self_user_id, contact_list, .. } = &mut self.screen else {
            return Task::none();
        };
        let uid = *self_user_id;
        let self_display_name = contact_list.display_name.clone();
        let Some(chat) = open_chats.get_mut(idx) else {
            return Task::none();
        };

        match msg {
            ChatMsg::InputChanged(v) => {
                let delta = if v.len() > chat.input.len() {
                    v[chat.input.len()..].to_string()
                } else {
                    chat.input = v;
                    return Task::none();
                };
                let msg_id = chat.current_msg_id;
                chat.input = v;

                nmp.send(Frame::new(
                    Opcode::MsgChunk,
                    payload::MsgChunk {
                        msg_id,
                        context_type: payload::ContextType::Dm,
                        context_id: chat.context_id,
                        author_id: uid,
                        delta: delta.clone(),
                    }.encode(),
                ));

                chat.apply_chunk(msg_id, uid, &self_display_name, &delta);
            }
            ChatMsg::Complete => {
                if !chat.input.is_empty() {
                    let msg_id = chat.current_msg_id;
                    let content = chat.input.clone();

                    nmp.send(Frame::new(
                        Opcode::MsgComplete,
                        payload::MsgComplete {
                            msg_id,
                            context_type: payload::ContextType::Dm,
                            context_id: chat.context_id,
                            author_id: uid,
                            content: content.clone(),
                        }.encode(),
                    ));

                    chat.complete_message(msg_id, &content);
                }
            }
            ChatMsg::Close => {
                open_chats.remove(idx);
                *active_chat = if open_chats.is_empty() { None } else { Some(0) };
            }
        }
        Task::none()
    }
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
