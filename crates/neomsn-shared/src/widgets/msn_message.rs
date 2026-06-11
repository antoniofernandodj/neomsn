use iced::{
    widget::{column, row, text, text::Shaping, Space},
    Element, Font, Length, Padding,
};
use crate::domain::message::{Message, MessageStatus};
use super::theme::MsnTheme;

/// Classic MSN rendering of one message:
///
/// ```text
/// Fulano diz:
///     mensagem em texto
/// ```
///
/// System lines (nil author) are rendered as gray italic notices, e.g.
/// "Fulano entrou na conversa.".
pub fn msn_message<Msg>(message: &Message) -> Element<'_, Msg>
where
    Msg: 'static,
{
    if message.author_id.is_nil() {
        return text(&message.content)
            .size(12)
            .font(Font { style: iced::font::Style::Italic, ..Font::DEFAULT })
            .color(MsnTheme::SYSTEM_MSG)
            .into();
    }

    let says = text(format!("{} diz:", message.author_display_name))
        .size(12)
        .color(MsnTheme::SAYS);

    let body: Element<'_, Msg> = if message.content.is_empty() {
        // First chunk not in yet — show the classic pencil placeholder.
        text("✎ …").size(13).color(MsnTheme::TEXT_SECONDARY).into()
    } else if message.status == MessageStatus::Streaming {
        // Live text with a typing caret at the end.
        row![
            text(&message.content).size(13).color(MsnTheme::TEXT_PRIMARY).shaping(Shaping::Advanced),
            text("▌").size(13).color(MsnTheme::ACCENT),
        ]
        .into()
    } else {
        text(&message.content).size(13).color(MsnTheme::TEXT_PRIMARY).shaping(Shaping::Advanced).into()
    };

    let indented = row![Space::with_width(Length::Fixed(14.0)), body];

    column![says, indented]
        .spacing(1)
        .padding(Padding { bottom: 4.0, ..Padding::ZERO })
        .into()
}
