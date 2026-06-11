use iced::{
    widget::{button, column, container, row, scrollable, text, text_input, Space},
    Alignment, Element, Font, Length, Padding,
};
use neomsn_shared::{
    domain::{
        contact::Contact,
        message::{ContextType, Message, MessageStatus},
        user::PresenceStatus,
    },
    widgets::{buddy_avatar, buddy_icon, emoji_picker, msn_message, theme::MsnTheme},
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum ChatMsg {
    InputChanged(String),
    Complete,
    ToggleInvite,
    Invite(Uuid),
    ToggleEmoji,
    EmojiPicked(&'static str),
    Nudge,
}

/// Number of animation ticks a nudge shake lasts (see `SHAKE_PATTERN`).
pub const SHAKE_TICKS: u8 = 14;

/// Content offsets cycled through while shaking, in logical pixels.
const SHAKE_PATTERN: [(f32, f32); 4] = [(8.0, 0.0), (0.0, 8.0), (8.0, 8.0), (0.0, 0.0)];

/// One open conversation window (DM or group/room).
pub struct ChatScreen {
    pub context_id: Uuid,
    pub context_type: ContextType,
    /// Everyone in the conversation, including self: (user_id, display_name).
    pub members: Vec<(Uuid, String)>,
    pub self_user_id: Uuid,
    /// Current message being typed (may be streaming to others).
    pub current_msg_id: Uuid,
    pub input: String,
    pub messages: Vec<Message>,
    pub invite_open: bool,
    pub emoji_open: bool,
    pub last_received_at: Option<String>,
    /// Remaining nudge animation ticks; 0 = not shaking.
    pub shake: u8,
}

impl ChatScreen {
    pub fn new_dm(
        conversation_id: Uuid,
        peer_id: Uuid,
        peer_display_name: String,
        self_user_id: Uuid,
        self_display_name: String,
    ) -> Self {
        Self {
            context_id: conversation_id,
            context_type: ContextType::Dm,
            members: vec![(self_user_id, self_display_name), (peer_id, peer_display_name)],
            self_user_id,
            current_msg_id: Uuid::new_v4(),
            input: String::new(),
            messages: Vec::new(),
            invite_open: false,
            emoji_open: false,
            last_received_at: None,
            shake: 0,
        }
    }

    pub fn new_room(room_id: Uuid, members: Vec<(Uuid, String)>, self_user_id: Uuid) -> Self {
        Self {
            context_id: room_id,
            context_type: ContextType::Room,
            members,
            self_user_id,
            current_msg_id: Uuid::new_v4(),
            input: String::new(),
            messages: Vec::new(),
            invite_open: false,
            emoji_open: false,
            last_received_at: None,
            shake: 0,
        }
    }

    /// Names of everyone except self — used as the window title ("Para:" line).
    pub fn title(&self) -> String {
        let names: Vec<&str> = self.members.iter()
            .filter(|(id, _)| *id != self.self_user_id)
            .map(|(_, name)| name.as_str())
            .collect();
        if names.is_empty() { "Conversa".into() } else { names.join(", ") }
    }

    pub fn member_name(&self, user_id: Uuid) -> Option<&str> {
        self.members.iter()
            .find(|(id, _)| *id == user_id)
            .map(|(_, name)| name.as_str())
    }

    /// Convert this DM window into a group conversation (room) in place.
    pub fn upgrade_to_room(&mut self, room_id: Uuid, members: Vec<(Uuid, String)>) {
        self.context_id = room_id;
        self.context_type = ContextType::Room;
        self.members = members;
    }

    pub fn push_system(&mut self, line: String) {
        let mut msg = Message::new_streaming(
            Uuid::new_v4(),
            self.context_type,
            self.context_id,
            Uuid::nil(),
            String::new(),
        );
        msg.content = line;
        msg.complete();
        self.messages.push(msg);
    }

    /// Apply an incoming chunk (from server broadcast or own echo).
    pub fn apply_chunk(
        &mut self,
        msg_id: Uuid,
        author_id: Uuid,
        fallback_name: &str,
        truncate_to: usize,
        delta: &str,
    ) {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == msg_id) {
            msg.apply_chunk(truncate_to, delta);
        } else {
            let name = self.member_name(author_id)
                .unwrap_or(fallback_name)
                .to_string();
            let mut msg = Message::new_streaming(
                msg_id,
                self.context_type,
                self.context_id,
                author_id,
                name,
            );
            msg.apply_chunk(truncate_to, delta);
            self.messages.push(msg);
        }
    }

    pub fn complete_message(&mut self, msg_id: Uuid, content: &str) {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == msg_id) {
            msg.content = content.to_string();
            msg.complete();
            if msg.author_id != self.self_user_id {
                self.last_received_at = Some(chrono::Local::now().format("%H:%M").to_string());
            }
        }
        // If it was our own message, reset input and generate new id.
        if self.current_msg_id == msg_id {
            self.input.clear();
            self.current_msg_id = Uuid::new_v4();
        }
    }

    pub fn delete_message(&mut self, msg_id: Uuid) {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == msg_id) {
            msg.delete();
        }
    }

    /// Prepend completed history loaded from the server, skipping messages we
    /// already have (e.g. the live ones that arrived before the sync response).
    pub fn prepend_history(&mut self, history: Vec<(Uuid, Uuid, String, String)>) {
        let mut older: Vec<Message> = Vec::new();
        for (msg_id, author_id, author_name, content) in history {
            if self.messages.iter().any(|m| m.id == msg_id) { continue; }
            let mut msg = Message::new_streaming(
                msg_id,
                self.context_type,
                self.context_id,
                author_id,
                author_name,
            );
            msg.content = content;
            msg.complete();
            older.push(msg);
        }
        older.append(&mut self.messages);
        self.messages = older;
    }

    fn typing_user(&self) -> Option<&str> {
        self.messages.iter().rev()
            .find(|m| m.status == MessageStatus::Streaming && m.author_id != self.self_user_id)
            .map(|m| m.author_display_name.as_str())
    }

    pub fn view<'a>(&'a self, contacts: &'a [Contact]) -> Element<'a, ChatMsg> {
        // ── "Para:" bar ──
        let to_bar = container(
            row![
                text("Para:").size(12).color(MsnTheme::TEXT_SECONDARY),
                text(self.title()).size(12).color(MsnTheme::ACCENT),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .style(|_| MsnTheme::header_style())
        .padding(Padding::from([5, 10]))
        .width(Length::Fill);

        // ── Toolbar ──
        let invite_btn = button(text("Convidar").size(12))
            .style(MsnTheme::toolbar_button)
            .on_press(ChatMsg::ToggleInvite)
            .padding(Padding::from([4, 8]));

        let decorative = |label: &'static str| {
            button(text(label).size(12))
                .style(|theme, _status| {
                    let mut s = MsnTheme::toolbar_button(theme, button::Status::Active);
                    s.text_color = MsnTheme::TEXT_SECONDARY;
                    s
                })
                .padding(Padding::from([4, 8]))
        };

        let nudge_btn = button(text("Chamar atenção").size(12))
            .style(MsnTheme::toolbar_button)
            .on_press(ChatMsg::Nudge)
            .padding(Padding::from([4, 8]));

        let toolbar = container(
            row![
                invite_btn,
                nudge_btn,
                decorative("Enviar arquivos"),
                decorative("Webcam"),
                decorative("Jogos"),
            ]
            .spacing(2)
            .align_y(Alignment::Center),
        )
        .style(|_| container::Style {
            background: Some(iced::Background::Color(MsnTheme::TOOLBAR_BG)),
            border: iced::Border {
                color: MsnTheme::FRAME_BORDER,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .padding(Padding::from([2, 6]))
        .width(Length::Fill);

        // ── Invite panel (contact picker) ──
        let invite_panel: Element<'a, ChatMsg> = if self.invite_open {
            let member_ids: Vec<Uuid> = self.members.iter().map(|(id, _)| *id).collect();
            let candidates: Vec<&Contact> = contacts.iter()
                .filter(|c| c.presence != PresenceStatus::Offline)
                .filter(|c| !member_ids.contains(&c.user_id))
                .collect();

            let mut col = column![
                text("Convidar para esta conversa:").size(12).color(MsnTheme::GROUP)
            ]
            .spacing(2);

            if candidates.is_empty() {
                col = col.push(
                    text("Nenhum contato online disponível.")
                        .size(12)
                        .color(MsnTheme::TEXT_SECONDARY),
                );
            }
            for c in candidates {
                col = col.push(
                    button(
                        row![
                            buddy_icon(c.presence, 15.0),
                            text(&c.display_name).size(12),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center),
                    )
                    .style(MsnTheme::toolbar_button)
                    .on_press(ChatMsg::Invite(c.user_id))
                    .padding(Padding::from([3, 8]))
                    .width(Length::Fill),
                );
            }

            container(col)
                .style(|_| MsnTheme::panel_style())
                .padding(8)
                .width(Length::Fill)
                .into()
        } else {
            Space::with_height(0).into()
        };

        // ── Message history ──
        let mut msg_col = column![].spacing(4).padding(Padding::from([8, 10]));
        for msg in &self.messages {
            if msg.status == MessageStatus::Deleted { continue; }
            msg_col = msg_col.push(msn_message(msg));
        }

        let history = container(
            scrollable(msg_col)
                .height(Length::Fill)
                .width(Length::Fill)
                .anchor_bottom(),
        )
        .style(|_| MsnTheme::panel_style())
        .height(Length::Fill)
        .width(Length::Fill);

        // ── Avatar side panel: peer on top, self at the bottom ──
        let peer_status = if self.context_type == ContextType::Room {
            PresenceStatus::Online
        } else {
            self.members.iter()
                .find(|(id, _)| *id != self.self_user_id)
                .and_then(|(id, _)| contacts.iter().find(|c| c.user_id == *id))
                .map(|c| c.presence)
                .unwrap_or(PresenceStatus::Online)
        };

        let avatars = column![
            buddy_avatar(peer_status, 96.0),
            Space::with_height(Length::Fill),
            buddy_avatar(PresenceStatus::Online, 96.0),
        ]
        .align_x(Alignment::Center);

        let body = row![history, avatars]
            .spacing(8)
            .height(Length::Fill)
            .padding(Padding::from([8, 10]));

        // ── Input panel ──
        let mini_label = |label: &'static str| {
            text(label).size(11).color(MsnTheme::TEXT_SECONDARY)
        };
        let emoji_btn = button(
            text("Emojis")
                .size(11)
                .color(if self.emoji_open { MsnTheme::ACCENT } else { MsnTheme::TEXT_SECONDARY }),
        )
        .style(MsnTheme::toolbar_button)
        .on_press(ChatMsg::ToggleEmoji)
        .padding(Padding::from([1, 4]));

        let mini_toolbar = row![
            mini_label("Fonte"),
            emoji_btn,
            mini_label("Winks"),
            mini_label("Fundos"),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        let emoji_panel: Element<'a, ChatMsg> = if self.emoji_open {
            emoji_picker(ChatMsg::EmojiPicked)
        } else {
            Space::with_height(0).into()
        };

        let input_field = text_input("", &self.input)
            .on_input(ChatMsg::InputChanged)
            .on_submit(ChatMsg::Complete)
            .padding(6)
            .size(13);

        let complete_btn = button(text("Concluir").size(13))
            .style(MsnTheme::classic_button)
            .on_press(ChatMsg::Complete)
            .padding(Padding::from([10, 16]));

        let input_panel = container(
            column![
                mini_toolbar,
                emoji_panel,
                row![input_field, complete_btn]
                    .spacing(6)
                    .align_y(Alignment::Center),
            ]
            .spacing(4),
        )
        .style(|_| MsnTheme::panel_style())
        .padding(Padding::from([6, 8]))
        .width(Length::Fill);

        let input_area = container(input_panel).padding(Padding::from([0, 10]));

        // ── Status bar ──
        let status_line = if let Some(name) = self.typing_user() {
            format!("✎ {name} está digitando…")
        } else if let Some(at) = &self.last_received_at {
            format!("Última mensagem recebida às {at}.")
        } else {
            String::new()
        };

        let status_bar = container(
            text(status_line)
                .size(11)
                .font(Font { style: iced::font::Style::Italic, ..Font::DEFAULT })
                .color(MsnTheme::TEXT_SECONDARY),
        )
        .style(|_| container::Style {
            background: Some(MsnTheme::vertical_gradient(
                MsnTheme::HEADER_TOP,
                MsnTheme::HEADER_BOTTOM,
            )),
            ..Default::default()
        })
        .padding(Padding::from([3, 10]))
        .width(Length::Fill);

        let root = column![
            to_bar,
            toolbar,
            invite_panel,
            body,
            input_area,
            Space::with_height(6),
            status_bar,
        ];

        if self.shake == 0 {
            root.into()
        } else {
            // Nudge animation: jolt the whole content around.
            let (dx, dy) = SHAKE_PATTERN[(self.shake % 4) as usize];
            container(root)
                .padding(Padding { top: dy, right: 0.0, bottom: 0.0, left: dx })
                .into()
        }
    }
}
