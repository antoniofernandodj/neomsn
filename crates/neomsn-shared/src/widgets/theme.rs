use iced::Color;

/// MSN Messenger–inspired color palette.
pub struct MsnTheme;

impl MsnTheme {
    // Blues
    pub const HEADER_TOP: Color    = Color { r: 0.24, g: 0.48, b: 0.78, a: 1.0 };
    pub const HEADER_BOTTOM: Color = Color { r: 0.14, g: 0.32, b: 0.62, a: 1.0 };
    pub const ACCENT: Color        = Color { r: 0.18, g: 0.40, b: 0.72, a: 1.0 };

    // Backgrounds
    pub const BG: Color            = Color { r: 0.97, g: 0.97, b: 0.98, a: 1.0 };
    pub const BG_PANEL: Color      = Color { r: 1.00, g: 1.00, b: 1.00, a: 1.0 };
    pub const BG_HOVER: Color      = Color { r: 0.88, g: 0.93, b: 0.98, a: 1.0 };

    // Text
    pub const TEXT_PRIMARY: Color  = Color { r: 0.10, g: 0.10, b: 0.10, a: 1.0 };
    pub const TEXT_SECONDARY: Color= Color { r: 0.45, g: 0.45, b: 0.50, a: 1.0 };
    pub const TEXT_ON_ACCENT: Color= Color::WHITE;

    // Presence indicators
    pub const ONLINE: Color    = Color { r: 0.18, g: 0.72, b: 0.27, a: 1.0 };
    pub const AWAY: Color      = Color { r: 0.95, g: 0.80, b: 0.10, a: 1.0 };
    pub const BUSY: Color      = Color { r: 0.85, g: 0.20, b: 0.20, a: 1.0 };
    pub const INVISIBLE: Color = Color { r: 0.70, g: 0.70, b: 0.70, a: 1.0 };
    pub const OFFLINE: Color   = Color { r: 0.55, g: 0.55, b: 0.55, a: 1.0 };

    // Bubbles
    pub const BUBBLE_SELF: Color  = Color { r: 0.85, g: 0.92, b: 1.00, a: 1.0 };
    pub const BUBBLE_OTHER: Color = Color { r: 0.95, g: 0.95, b: 0.95, a: 1.0 };

    pub fn iced() -> iced::Theme {
        iced::Theme::custom(
            "NeoMSN".to_string(),
            iced::theme::Palette {
                background: Self::BG,
                text: Self::TEXT_PRIMARY,
                primary: Self::ACCENT,
                success: Self::ONLINE,
                danger: Self::BUSY,
            },
        )
    }
}
