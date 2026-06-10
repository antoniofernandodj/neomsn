use iced::{
    widget::{button, column, container, row, scrollable, text, text_input, Space},
    Alignment, Element, Length, Padding,
};
use neomsn_shared::{
    domain::{contact::Contact, user::PresenceStatus},
    widgets::{contact_item, theme::MsnTheme},
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

    pub fn view(&self) -> Element<ContactListMsg> {
        // ── Header ──
        let name = text(&self.display_name).size(14).color(MsnTheme::TEXT_ON_ACCENT);
        let pm   = text(&self.personal_message).size(11).color(iced::Color { a: 0.75, ..MsnTheme::TEXT_ON_ACCENT });

        let status_btn = button(
            text(self.status.label()).size(11).color(MsnTheme::TEXT_ON_ACCENT)
        )
        .style(button::text)
        .padding(0);

        let header = container(
            column![name, pm, status_btn].spacing(2)
        )
        .style(|_| container::Style {
            background: Some(iced::Background::Color(MsnTheme::HEADER_TOP)),
            ..Default::default()
        })
        .padding(Padding::from([12, 14]))
        .width(Length::Fill);

        // ── Search / Add ──
        let search = text_input("Pesquisar contatos…", &self.search)
            .on_input(ContactListMsg::SearchChanged)
            .padding(6)
            .size(13);

        let add_row = row![
            text_input("Adicionar pelo usuário…", &self.add_input)
                .on_input(ContactListMsg::AddContactInput)
                .on_submit(ContactListMsg::AddContact)
                .padding(6)
                .size(13),
            button(text("+").size(14))
                .on_press(ContactListMsg::AddContact)
                .padding(Padding::from([4, 10])),
        ]
        .spacing(4)
        .align_y(Alignment::Center);

        // ── Pending requests ──
        let mut list = column![].spacing(0);

        if !self.pending_requests.is_empty() {
            list = list.push(section_header(
                format!("Solicitações ({})", self.pending_requests.len()),
                iced::Color { r: 0.95, g: 0.88, b: 0.70, a: 1.0 },
            ));
            for req in &self.pending_requests {
                list = list.push(pending_request_row(req));
            }
            list = list.push(Space::with_height(4));
        }

        // ── Contact list ──
        let query = self.search.to_lowercase();

        let online: Vec<&Contact> = self.contacts.iter()
            .filter(|c| c.presence != PresenceStatus::Offline && matches_search(c, &query))
            .collect();

        let offline: Vec<&Contact> = self.contacts.iter()
            .filter(|c| c.presence == PresenceStatus::Offline && matches_search(c, &query))
            .collect();

        if !online.is_empty() {
            list = list.push(section_header(
                format!("Online ({})", online.len()),
                iced::Color { r: 0.91, g: 0.93, b: 0.96, a: 1.0 },
            ));
            for c in online {
                list = list.push(contact_item(c, ContactListMsg::OpenChat(c.user_id)));
            }
        }
        if !offline.is_empty() {
            list = list.push(Space::with_height(4));
            list = list.push(section_header(
                format!("Offline ({})", offline.len()),
                iced::Color { r: 0.91, g: 0.93, b: 0.96, a: 1.0 },
            ));
            for c in offline {
                list = list.push(contact_item(c, ContactListMsg::OpenChat(c.user_id)));
            }
        }

        let scrolled = scrollable(list).height(Length::Fill);

        column![
            header,
            container(column![search, Space::with_height(4), add_row].spacing(0))
                .padding(Padding::from([6, 8]))
                .width(Length::Fill),
            scrolled,
        ]
        .into()
    }
}

fn pending_request_row(req: &PendingRequest) -> Element<'_, ContactListMsg> {
    let uid = req.user_id;

    let name = text(&req.display_name).size(13).color(MsnTheme::TEXT_PRIMARY);
    let username = text(format!("@{}", req.username)).size(11).color(MsnTheme::TEXT_SECONDARY);

    let accept_btn = button(text("Aceitar").size(12).color(MsnTheme::ONLINE))
        .style(button::text)
        .on_press(ContactListMsg::AcceptRequest(uid))
        .padding(Padding::from([2, 6]));

    let reject_btn = button(text("Recusar").size(12).color(MsnTheme::BUSY))
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
        background: Some(iced::Background::Color(iced::Color { r: 1.0, g: 0.97, b: 0.88, a: 1.0 })),
        border: iced::Border {
            width: 0.0,
            color: iced::Color::TRANSPARENT,
            radius: 0.0.into(),
        },
        ..Default::default()
    })
    .padding(Padding::from([6, 10]))
    .width(Length::Fill)
    .into()
}

fn section_header(label: String, bg: iced::Color) -> Element<'static, ContactListMsg> {
    container(text(label).size(11).color(MsnTheme::TEXT_SECONDARY))
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(bg)),
            ..Default::default()
        })
        .padding(Padding::from([3, 10]))
        .width(Length::Fill)
        .into()
}

fn matches_search(c: &Contact, query: &str) -> bool {
    if query.is_empty() { return true; }
    c.display_name.to_lowercase().contains(query) || c.username.to_lowercase().contains(query)
}
