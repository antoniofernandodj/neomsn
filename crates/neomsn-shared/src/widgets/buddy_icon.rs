use iced::{
    mouse,
    widget::canvas::{self, Canvas, Geometry, Path, Stroke},
    Color, Element, Length, Point, Rectangle, Renderer, Size, Theme,
};
use crate::domain::user::PresenceStatus;
use super::theme::MsnTheme;

/// The classic MSN "buddy" silhouette (head + torso), tinted by presence.
pub struct BuddyIcon {
    color: Color,
}

impl BuddyIcon {
    pub fn for_status(status: PresenceStatus) -> Self {
        let color = match status {
            PresenceStatus::Online    => MsnTheme::ONLINE,
            PresenceStatus::Away      => MsnTheme::AWAY,
            PresenceStatus::Busy      => MsnTheme::BUSY,
            PresenceStatus::Invisible => MsnTheme::INVISIBLE,
            PresenceStatus::Offline   => MsnTheme::OFFLINE,
        };
        Self { color }
    }
}

impl<Message> canvas::Program<Message> for BuddyIcon {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let w = bounds.width;
        let h = bounds.height;

        let outline = Color {
            r: self.color.r * 0.55,
            g: self.color.g * 0.55,
            b: self.color.b * 0.55,
            a: 1.0,
        };
        let stroke = Stroke::default()
            .with_color(outline)
            .with_width((w * 0.04).max(1.0));

        // Head
        let head = Path::circle(Point::new(w * 0.50, h * 0.30), w * 0.21);
        frame.fill(&head, self.color);
        frame.stroke(&head, stroke);

        // Torso: tall rounded rectangle, bottom clipped by the canvas edge.
        let torso = Path::rounded_rectangle(
            Point::new(w * 0.18, h * 0.56),
            Size::new(w * 0.64, h * 0.60),
            (w * 0.30).into(),
        );
        frame.fill(&torso, self.color);
        frame.stroke(&torso, stroke);

        vec![frame.into_geometry()]
    }
}

/// Small presence buddy for contact rows and status selectors.
pub fn buddy_icon<Message: 'static>(status: PresenceStatus, size: f32) -> Element<'static, Message> {
    Canvas::new(BuddyIcon::for_status(status))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .into()
}

/// Large framed buddy used as avatar placeholder (chat window side panels,
/// login screen) — white card with the thin blue MSN border.
pub fn buddy_avatar<Message: 'static>(status: PresenceStatus, size: f32) -> Element<'static, Message> {
    iced::widget::container(buddy_icon(status, size * 0.74))
        .style(|_| iced::widget::container::Style {
            background: Some(MsnTheme::vertical_gradient(
                Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 },
                Color { r: 0.87, g: 0.92, b: 0.97, a: 1.0 },
            )),
            border: iced::Border {
                color: MsnTheme::FRAME_BORDER,
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .center_x(Length::Fixed(size))
        .center_y(Length::Fixed(size))
        .into()
}
