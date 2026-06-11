use iced::{gradient, widget::button, Background, Border, Color, Radians, Shadow};

/// Windows Live Messenger (8.x / 2009) inspired palette and styles.
pub struct MsnTheme;

impl MsnTheme {
    // Window chrome blues
    pub const HEADER_TOP: Color    = Color { r: 0.95, g: 0.97, b: 1.00, a: 1.0 }; // #F2F8FF
    pub const HEADER_BOTTOM: Color = Color { r: 0.76, g: 0.86, b: 0.96, a: 1.0 }; // #C2DBF5
    pub const ACCENT: Color        = Color { r: 0.11, g: 0.37, b: 0.66, a: 1.0 }; // MSN blue
    pub const FRAME_BORDER: Color  = Color { r: 0.58, g: 0.70, b: 0.84, a: 1.0 }; // #93B2D6
    pub const TOOLBAR_BG: Color    = Color { r: 0.91, g: 0.95, b: 0.99, a: 1.0 };

    // Backgrounds
    pub const BG: Color            = Color { r: 0.94, g: 0.96, b: 0.99, a: 1.0 };
    pub const BG_PANEL: Color      = Color { r: 1.00, g: 1.00, b: 1.00, a: 1.0 };
    pub const BG_HOVER: Color      = Color { r: 0.85, g: 0.91, b: 0.97, a: 1.0 };

    // Text
    pub const TEXT_PRIMARY: Color   = Color { r: 0.10, g: 0.10, b: 0.10, a: 1.0 };
    pub const TEXT_SECONDARY: Color = Color { r: 0.45, g: 0.45, b: 0.50, a: 1.0 };
    pub const TEXT_ON_ACCENT: Color = Color::WHITE;
    /// "Fulano diz:" line color, like classic MSN.
    pub const SAYS: Color           = Color { r: 0.35, g: 0.35, b: 0.38, a: 1.0 };
    /// Group headers in the contact list ("Online (2)").
    pub const GROUP: Color          = Color { r: 0.11, g: 0.37, b: 0.66, a: 1.0 };
    pub const SYSTEM_MSG: Color     = Color { r: 0.50, g: 0.52, b: 0.56, a: 1.0 };

    // Presence indicators (buddy colors)
    pub const ONLINE: Color    = Color { r: 0.42, g: 0.72, b: 0.25, a: 1.0 };
    pub const AWAY: Color      = Color { r: 0.95, g: 0.77, b: 0.18, a: 1.0 };
    pub const BUSY: Color      = Color { r: 0.83, g: 0.25, b: 0.20, a: 1.0 };
    pub const INVISIBLE: Color = Color { r: 0.68, g: 0.70, b: 0.73, a: 1.0 };
    pub const OFFLINE: Color   = Color { r: 0.68, g: 0.70, b: 0.73, a: 1.0 };

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

    // ── Reusable style helpers ────────────────────────────────────────────────

    pub fn vertical_gradient(top: Color, bottom: Color) -> Background {
        Background::Gradient(
            gradient::Linear::new(Radians(std::f32::consts::PI))
                .add_stop(0.0, top)
                .add_stop(1.0, bottom)
                .into(),
        )
    }

    /// Soft blue header banner (contact list header, chat "Para:" bar).
    pub fn header_style() -> iced::widget::container::Style {
        iced::widget::container::Style {
            background: Some(Self::vertical_gradient(Self::HEADER_TOP, Self::HEADER_BOTTOM)),
            border: Border {
                color: Self::FRAME_BORDER,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        }
    }

    /// White panel with the classic thin blue border.
    pub fn panel_style() -> iced::widget::container::Style {
        iced::widget::container::Style {
            background: Some(Background::Color(Self::BG_PANEL)),
            border: Border {
                color: Self::FRAME_BORDER,
                width: 1.0,
                radius: 2.0.into(),
            },
            ..Default::default()
        }
    }

    /// Classic XP-era button: light gradient, blue border, rounded corners.
    pub fn classic_button(_theme: &iced::Theme, status: button::Status) -> button::Style {
        let (top, bottom) = match status {
            button::Status::Hovered => (
                Color { r: 1.00, g: 1.00, b: 1.00, a: 1.0 },
                Color { r: 0.80, g: 0.89, b: 0.97, a: 1.0 },
            ),
            button::Status::Pressed => (
                Color { r: 0.78, g: 0.86, b: 0.94, a: 1.0 },
                Color { r: 0.88, g: 0.93, b: 0.98, a: 1.0 },
            ),
            _ => (
                Color { r: 0.99, g: 0.99, b: 1.00, a: 1.0 },
                Color { r: 0.85, g: 0.91, b: 0.97, a: 1.0 },
            ),
        };
        button::Style {
            background: Some(Self::vertical_gradient(top, bottom)),
            text_color: Self::TEXT_PRIMARY,
            border: Border {
                color: Self::FRAME_BORDER,
                width: 1.0,
                radius: 3.0.into(),
            },
            shadow: Shadow::default(),
        }
    }

    /// Flat toolbar button: transparent, light blue when hovered.
    pub fn toolbar_button(_theme: &iced::Theme, status: button::Status) -> button::Style {
        button::Style {
            background: Some(Background::Color(match status {
                button::Status::Hovered | button::Status::Pressed => Self::BG_HOVER,
                _ => Color::TRANSPARENT,
            })),
            text_color: Self::ACCENT,
            border: Border {
                color: match status {
                    button::Status::Hovered | button::Status::Pressed => Self::FRAME_BORDER,
                    _ => Color::TRANSPARENT,
                },
                width: 1.0,
                radius: 3.0.into(),
            },
            shadow: Shadow::default(),
        }
    }
}
