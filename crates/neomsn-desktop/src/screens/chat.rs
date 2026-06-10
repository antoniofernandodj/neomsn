use iced::{
    widget::{button, column, container, row, scrollable, text, text_input, Space},
    Alignment, Element, Length, Padding,
};
use neomsn_shared::{
    domain::message::{ContextType, Message, MessageStatus},
    widgets::{chat_bubble, theme::MsnTheme},
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum ChatMsg {
    InputChanged(String),
    Complete,
    Close,
}

pub struct ChatScreen {
    pub context_id: Uuid,
    pub context_type: ContextType,
    pub peer_display_name: String,
    pub self_user_id: Uuid,
    /// Current message being typed (may be streaming to others).
    pub current_msg_id: Uuid,
    pub input: String,
    pub messages: Vec<Message>,
}

impl ChatScreen {
    pub fn new(
        context_id: Uuid,
        context_type: ContextType,
        peer_display_name: String,
        self_user_id: Uuid,
    ) -> Self {
        Self {
            context_id,
            context_type,
            peer_display_name,
            self_user_id,
            current_msg_id: Uuid::new_v4(),
            input: String::new(),
            messages: Vec::new(),
        }
    }

    /// Apply an incoming chunk (from server broadcast or own echo).
    pub fn apply_chunk(&mut self, msg_id: Uuid, author_id: Uuid, author_name: &str, delta: &str) {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == msg_id) {
            msg.apply_chunk(delta);
        } else {
            let mut msg = Message::new_streaming(
                msg_id,
                self.context_type,
                self.context_id,
                author_id,
                author_name.to_string(),
            );
            msg.apply_chunk(delta);
            self.messages.push(msg);
        }
    }

    pub fn complete_message(&mut self, msg_id: Uuid, content: &str) {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == msg_id) {
            msg.content = content.to_string();
            msg.complete();
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

    pub fn view(&self) -> Element<ChatMsg> {
        // ── Title bar ──
        let title = text(&self.peer_display_name).size(13).color(MsnTheme::TEXT_ON_ACCENT);
        let close_btn = button(text("✕").size(12).color(MsnTheme::TEXT_ON_ACCENT))
            .style(button::text)
            .on_press(ChatMsg::Close)
            .padding(0);

        let titlebar = container(
            row![title, Space::with_width(Length::Fill), close_btn].align_y(Alignment::Center)
        )
        .style(|_| container::Style {
            background: Some(iced::Background::Color(MsnTheme::HEADER_TOP)),
            ..Default::default()
        })
        .padding(Padding::from([8, 12]))
        .width(Length::Fill);

        // ── Message history ──
        let mut msg_col = column![].spacing(6).padding(Padding::from([8, 12]));
        for msg in &self.messages {
            if msg.status == MessageStatus::Deleted { continue; }
            let is_self = msg.author_id == self.self_user_id;
            msg_col = msg_col.push(chat_bubble(msg, is_self));
        }

        let history = scrollable(msg_col)
            .height(Length::Fill)
            .anchor_bottom();

        // ── Input area ──
        let input_field = text_input("Digite uma mensagem…", &self.input)
            .on_input(ChatMsg::InputChanged)
            .padding(8)
            .size(14);

        let complete_btn = button(text("Concluir").size(13))
            .on_press(ChatMsg::Complete)
            .padding(Padding::from([8, 14]));

        let input_row = container(
            row![input_field, complete_btn].spacing(6).align_y(Alignment::Center)
        )
        .style(|_| container::Style {
            background: Some(iced::Background::Color(MsnTheme::BG_PANEL)),
            border: iced::Border {
                width: 1.0,
                color: iced::Color { r: 0.80, g: 0.85, b: 0.92, a: 1.0 },
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .padding(Padding::from([6, 8]))
        .width(Length::Fill);

        column![titlebar, history, input_row].into()
    }
}
