use iced::{
    widget::{button, column, container, row, text},
    Alignment, Element, Length, Padding,
};
use crate::domain::contact::Contact;
use super::{status_dot, theme::MsnTheme};

/// A single row in the contact list — avatar placeholder, name, presence dot.
pub fn contact_item<Message>(contact: &Contact, on_open: Message) -> Element<'_, Message>
where
    Message: Clone + 'static,
{
    let dot = status_dot(contact.presence);

    let name = text(&contact.display_name)
        .size(14)
        .color(MsnTheme::TEXT_PRIMARY);

    let personal_msg = text(contact.presence.label())
        .size(11)
        .color(MsnTheme::TEXT_SECONDARY);

    let info = column![name, personal_msg].spacing(2);

    let row = row![dot, info]
        .spacing(8)
        .align_y(Alignment::Center);

    let inner = container(row)
        .padding(Padding::from([6, 10]))
        .width(Length::Fill);

    button(inner)
        .on_press(on_open)
        .style(|theme, status| {
            let mut s = button::text(theme, status);
            s.background = Some(iced::Background::Color(match status {
                button::Status::Hovered | button::Status::Pressed => MsnTheme::BG_HOVER,
                _ => iced::Color::TRANSPARENT,
            }));
            s
        })
        .width(Length::Fill)
        .into()
}
