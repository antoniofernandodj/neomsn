use iced::{
    widget::{button, column, container, pick_list, row, scrollable, text, text_input, Space},
    Alignment, Element, Font, Length, Padding,
};
use neomsn_shared::{
    domain::{contact::Contact, user::PresenceStatus},
    widgets::{buddy_avatar, contact_item, theme::MsnTheme},
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PendingRequest {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
}

#[derive(Debug, Clone)]
pub enum ContactListMsg {
    OpenChat(Uuid),
    SearchChanged(String),
    AddContactInput(String),
    AddContact,
    SetStatus(PresenceStatus),
    AcceptRequest(Uuid),
    RejectRequest(Uuid),
}

pub struct ContactListScreen {
    pub display_name: String,
    pub personal_message: String,
    pub status: PresenceStatus,
    pub contacts: Vec<Contact>,
    pub pending_requests: Vec<PendingRequest>,
    pub search: String,
    pub add_input: String,
}

impl ContactListScreen {
    pub fn new(display_name: String) -> Self {
        Self {
            display_name,
            personal_message: String::new(),
            status: PresenceStatus::Online,
            contacts: Vec::new(),
            pending_requests: Vec::new(),
            search: String::new(),
            add_input: String::new(),
        }
    }

    pub fn view(&self) -> Element<'_, ContactListMsg> {
        // ── Header: avatar + name + status selector + personal message ──
        let status_picker = pick_list(
            PresenceStatus::SELECTABLE,
            Some(self.status),
            ContactListMsg::SetStatus,
        )
        .text_size(11)
        .padding(Padding::from([1, 6]))
        .style(|theme, status| {
            let mut s = pick_list::default(theme, status);
            s.background = iced::Background::Color(iced::Color::TRANSPARENT);
            s.border = iced::Border {
                color: MsnTheme::FRAME_BORDER,
                width: 1.0,
                radius: 3.0.into(),
            };
            s.text_color = MsnTheme::ACCENT;
            s
        });

        let pm: Element<'_, ContactListMsg> = if self.personal_message.is_empty() {
            text("<Digite uma mensagem pessoal>")
                .size(11)
                .font(Font { style: iced::font::Style::Italic, ..Font::DEFAULT })
                .color(MsnTheme::TEXT_SECONDARY)
                .into()
        } else {
            text(&self.personal_message)
                .size(11)
                .font(Font { style: iced::font::Style::Italic, ..Font::DEFAULT })
                .color(MsnTheme::TEXT_SECONDARY)
                .into()
        };

        let header = container(
            row![
                buddy_avatar(self.status, 52.0),
                column![
                    text(&self.display_name).size(14).color(MsnTheme::TEXT_PRIMARY),
                    status_picker,
                    pm,
                ]
                .spacing(2),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .style(|_| MsnTheme::header_style())
        .padding(Padding::from([10, 12]))
        .width(Length::Fill);

        // ── Search / Add ──
        let search = text_input("Pesquisar contatos…", &self.search)
            .on_input(ContactListMsg::SearchChanged)
            .padding(5)
            .size(12);

        let add_row = row![
            text_input("Adicionar pelo usuário…", &self.add_input)
                .on_input(ContactListMsg::AddContactInput)
                .on_submit(ContactListMsg::AddContact)
                .padding(5)
                .size(12),
            button(text("+").size(13))
                .style(MsnTheme::classic_button)
                .on_press(ContactListMsg::AddContact)
                .padding(Padding::from([4, 10])),
        ]
        .spacing(4)
        .align_y(Alignment::Center);

        // ── Pending requests ──
        let mut list = column![].spacing(0);

        if !self.pending_requests.is_empty() {
            list = list.push(group_header(format!(
                "Solicitações ({})",
                self.pending_requests.len()
            )));
            for req in &self.pending_requests {
                list = list.push(pending_request_row(req));
            }
            list = list.push(Space::with_height(4));
        }

        // ── Contact groups ──
        let query = self.search.to_lowercase();

        let online: Vec<&Contact> = self.contacts.iter()
            .filter(|c| c.presence != PresenceStatus::Offline && matches_search(c, &query))
            .collect();

        let offline: Vec<&Contact> = self.contacts.iter()
            .filter(|c| c.presence == PresenceStatus::Offline && matches_search(c, &query))
            .collect();

        list = list.push(group_header(format!("▾ Online ({})", online.len())));
        for c in online {
            list = list.push(contact_item(c, ContactListMsg::OpenChat(c.user_id)));
        }
        list = list.push(Space::with_height(4));
        list = list.push(group_header(format!("▾ Offline ({})", offline.len())));
        for c in offline {
            list = list.push(contact_item(c, ContactListMsg::OpenChat(c.user_id)));
        }

        let scrolled = container(
            scrollable(list).height(Length::Fill).width(Length::Fill),
        )
        .style(|_| container::Style {
            background: Some(iced::Background::Color(MsnTheme::BG_PANEL)),
            border: iced::Border {
                color: MsnTheme::FRAME_BORDER,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .height(Length::Fill)
        .width(Length::Fill);

        column![
            header,
            container(column![search, Space::with_height(4), add_row])
                .padding(Padding::from([6, 8]))
                .width(Length::Fill),
            scrolled,
        ]
        .into()
    }
}

fn pending_request_row(req: &PendingRequest) -> Element<'_, ContactListMsg> {
    let uid = req.user_id;

    let name = text(&req.display_name).size(12).color(MsnTheme::TEXT_PRIMARY);
    let username = text(format!("@{}", req.username)).size(11).color(MsnTheme::TEXT_SECONDARY);

    let accept_btn = button(text("Aceitar").size(11).color(MsnTheme::ONLINE))
        .style(button::text)
        .on_press(ContactListMsg::AcceptRequest(uid))
        .padding(Padding::from([2, 6]));

    let reject_btn = button(text("Recusar").size(11).color(MsnTheme::BUSY))
        .style(button::text)
        .on_press(ContactListMsg::RejectRequest(uid))
        .padding(Padding::from([2, 6]));

    container(
        row![
            column![name, username].spacing(1).width(Length::Fill),
            accept_btn,
            reject_btn,
        ]
        .align_y(Alignment::Center)
        .spacing(4),
    )
    .style(|_| container::Style {
        background: Some(iced::Background::Color(iced::Color {
            r: 1.0, g: 0.97, b: 0.88, a: 1.0,
        })),
        ..Default::default()
    })
    .padding(Padding::from([5, 10]))
    .width(Length::Fill)
    .into()
}

fn group_header(label: String) -> Element<'static, ContactListMsg> {
    container(text(label).size(12).color(MsnTheme::GROUP))
        .padding(Padding::from([4, 8]))
        .width(Length::Fill)
        .into()
}

fn matches_search(c: &Contact, query: &str) -> bool {
    if query.is_empty() { return true; }
    c.display_name.to_lowercase().contains(query) || c.username.to_lowercase().contains(query)
}
