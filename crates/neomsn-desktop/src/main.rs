mod app;
mod net;
mod screens;
mod sound;

use app::App;

fn main() -> iced::Result {
    iced::daemon(App::title, App::update, App::view)
        .subscription(App::subscription)
        .theme(App::theme)
        .run_with(App::new)
}
