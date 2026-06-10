use iced::{
    widget::container,
    Border, Color, Element, Length,
};
use crate::domain::user::PresenceStatus;
use super::theme::MsnTheme;

/// A small colored circle indicating presence status.
pub fn status_dot<Message>(status: PresenceStatus) -> Element<'static, Message>
where
    Message: 'static,
{
    let color = match status {
        PresenceStatus::Online    => MsnTheme::ONLINE,
        PresenceStatus::Away      => MsnTheme::AWAY,
        PresenceStatus::Busy      => MsnTheme::BUSY,
        PresenceStatus::Invisible => MsnTheme::INVISIBLE,
        PresenceStatus::Offline   => MsnTheme::OFFLINE,
    };

    container(iced::widget::Space::new(0, 0))
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(color)),
            border: Border { radius: 5.0.into(), ..Default::default() },
            ..Default::default()
        })
        .width(Length::Fixed(10.0))
        .height(Length::Fixed(10.0))
        .into()
}
