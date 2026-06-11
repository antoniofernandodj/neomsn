use iced::{
    widget::{button, column, container, row, text, text::Shaping},
    Element, Length, Padding,
};

use super::theme::MsnTheme;

/// MSN-classic friendly set, ordered roughly by how often people reach for them.
pub const EMOJIS: &[&str] = &[
    "🙂", "😀", "😂", "😉", "😛", "😮", "😎", "😍",
    "😢", "😭", "😡", "😇", "😈", "😴", "🤔", "😱",
    "❤️", "💔", "🌹", "👍", "👎", "👋", "🙏", "🎉",
    "⭐", "🎵", "☕", "🍺", "🎁", "📞", "✉️", "💡",
];

const PER_ROW: usize = 8;

/// Panel with a grid of emojis; clicking one emits `on_pick(emoji)`.
pub fn emoji_picker<'a, M: Clone + 'a>(
    on_pick: impl Fn(&'static str) -> M + 'a,
) -> Element<'a, M> {
    let mut grid = column![].spacing(2);
    for chunk in EMOJIS.chunks(PER_ROW) {
        let mut r = row![].spacing(2);
        for &emoji in chunk {
            r = r.push(
                button(text(emoji).size(18).shaping(Shaping::Advanced))
                    .style(MsnTheme::toolbar_button)
                    .on_press(on_pick(emoji))
                    .padding(Padding::from([2, 6])),
            );
        }
        grid = grid.push(r);
    }

    container(grid)
        .style(|_| MsnTheme::panel_style())
        .padding(6)
        .width(Length::Fill)
        .into()
}
