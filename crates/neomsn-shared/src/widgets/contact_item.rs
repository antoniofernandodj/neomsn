use iced::{
    widget::{button, container, row, text, Space},
    Alignment, Element, Length, Padding,
};
use crate::domain::{contact::Contact, user::PresenceStatus};
use super::{buddy_icon::buddy_icon, theme::MsnTheme};

/// A single row in the contact list, WLM style: indented buddy silhouette
/// tinted by presence, name, and the status in parentheses when not online.
pub fn contact_item<Message>(contact: &Contact, on_open: Message) -> Element<'_, Message>
where
    Message: Clone + 'static,
{
    let offline = contact.presence == PresenceStatus::Offline;

    let name = text(&contact.display_name)
        .size(13)
        .color(if offline { MsnTheme::TEXT_SECONDARY } else { MsnTheme::TEXT_PRIMARY });

    let status_suffix: Element<'_, Message> = match contact.presence {
        PresenceStatus::Online | PresenceStatus::Offline => Space::with_width(0).into(),
        s => text(format!(" ({})", s.label()))
            .size(12)
            .color(MsnTheme::TEXT_SECONDARY)
            .into(),
    };

    let inner = row![
        Space::with_width(Length::Fixed(14.0)),
        buddy_icon(contact.presence, 17.0),
        name,
        status_suffix,
    ]
    .spacing(5)
    .align_y(Alignment::Center);

    button(
        container(inner)
            .padding(Padding::from([3, 6]))
            .width(Length::Fill),
    )
    .on_press(on_open)
    .padding(0)
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
