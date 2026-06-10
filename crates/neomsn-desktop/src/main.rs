mod app;
mod net;
mod screens;

use app::App;
use neomsn_shared::widgets::theme::MsnTheme;

fn main() -> iced::Result {
    iced::application("NeoMSN", App::update, App::view)
        .subscription(App::subscription)
        .theme(|_| MsnTheme::iced())
        .window_size((320.0, 520.0))
        .run_with(App::new)
}
