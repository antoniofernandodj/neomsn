use iced::{
    widget::{column, container, row, text},
    Alignment, Element, Length, Padding,
};
use crate::domain::message::{Message, MessageStatus};
use super::theme::MsnTheme;

/// A chat bubble for one message. `is_self` flips alignment and color.
pub fn chat_bubble<Msg>(message: &Message, is_self: bool) -> Element<'_, Msg>
where
    Msg: 'static,
{
    let bubble_color = if is_self { MsnTheme::BUBBLE_SELF } else { MsnTheme::BUBBLE_OTHER };

    let body = if message.content.is_empty() && message.status == MessageStatus::Streaming {
        text("…").color(MsnTheme::TEXT_SECONDARY).size(14)
    } else {
        text(&message.content).size(14).color(MsnTheme::TEXT_PRIMARY)
    };

    let streaming_indicator = if message.status == MessageStatus::Streaming {
        text(" ●").size(10).color(MsnTheme::ACCENT)
    } else {
        text("").size(10)
    };

    let content_row = row![body, streaming_indicator].align_y(Alignment::End);

    let author = text(&message.author_display_name)
        .size(11)
        .color(MsnTheme::TEXT_SECONDARY);

    let bubble = container(
        column![author, content_row].spacing(2),
    )
    .style(move |_theme| container::Style {
        background: Some(iced::Background::Color(bubble_color)),
        border: iced::Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .padding(Padding::from([6, 10]))
    .max_width(420);

    let aligned = if is_self {
        row![iced::widget::Space::with_width(Length::Fill), bubble]
    } else {
        row![bubble, iced::widget::Space::with_width(Length::Fill)]
    };

    aligned.into()
}
