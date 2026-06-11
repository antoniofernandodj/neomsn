use iced::{
    widget::{button, column, container, text, text_input, Space},
    Alignment, Element, Length, Padding,
};
use neomsn_shared::{
    domain::user::PresenceStatus,
    widgets::{buddy_avatar, theme::MsnTheme},
};

#[derive(Debug, Clone)]
pub enum LoginMsg {
    UsernameChanged(String),
    PasswordChanged(String),
    DisplayNameChanged(String),
    Submit,
    ToggleSignup,
}

pub struct LoginScreen {
    pub username: String,
    pub password: String,
    pub display_name: String,
    pub signup_mode: bool,
    pub error: Option<String>,
    pub loading: bool,
}

impl LoginScreen {
    pub fn new() -> Self {
        Self {
            username: String::new(),
            password: String::new(),
            display_name: String::new(),
            signup_mode: false,
            error: None,
            loading: false,
        }
    }

    pub fn view(&self) -> Element<'_, LoginMsg> {
        // Big buddy avatar in a frame, like the WLM sign-in screen.
        let avatar = buddy_avatar(PresenceStatus::Online, 110.0);

        let field_label = |label: &'static str| {
            text(label).size(12).color(MsnTheme::TEXT_SECONDARY)
        };

        let username_field = text_input("Exemplo: fulano", &self.username)
            .on_input(LoginMsg::UsernameChanged)
            .on_submit(LoginMsg::Submit)
            .padding(6)
            .size(13);

        let password_field = text_input("", &self.password)
            .on_input(LoginMsg::PasswordChanged)
            .on_submit(LoginMsg::Submit)
            .secure(true)
            .padding(6)
            .size(13);

        let mut form = column![
            field_label("Usuário:"),
            username_field,
            Space::with_height(6),
            field_label("Senha:"),
            password_field,
        ]
        .spacing(2)
        .width(Length::Fixed(210.0));

        if self.signup_mode {
            form = form
                .push(Space::with_height(6))
                .push(field_label("Nome de exibição:"))
                .push(
                    text_input("Como seus amigos vão te ver", &self.display_name)
                        .on_input(LoginMsg::DisplayNameChanged)
                        .on_submit(LoginMsg::Submit)
                        .padding(6)
                        .size(13),
                );
        }

        let submit_label = if self.loading {
            "Aguarde…"
        } else if self.signup_mode {
            "Criar conta"
        } else {
            "Entrar"
        };
        let mut submit_btn = button(text(submit_label).size(13))
            .style(MsnTheme::classic_button)
            .padding(Padding::from([6, 28]));
        if !self.loading {
            submit_btn = submit_btn.on_press(LoginMsg::Submit);
        }

        let toggle_label = if self.signup_mode {
            "Já tenho uma conta"
        } else {
            "Criar uma conta"
        };
        let toggle_btn = button(text(toggle_label).size(12).color(MsnTheme::ACCENT))
            .style(button::text)
            .on_press(LoginMsg::ToggleSignup);

        let error_row: Element<'_, LoginMsg> = if let Some(e) = &self.error {
            text(e).size(12).color(MsnTheme::BUSY).into()
        } else {
            Space::with_height(0).into()
        };

        let body = column![
            Space::with_height(26),
            avatar,
            Space::with_height(18),
            form,
            Space::with_height(12),
            error_row,
            Space::with_height(8),
            submit_btn,
            Space::with_height(4),
            toggle_btn,
        ]
        .align_x(Alignment::Center)
        .width(Length::Fill);

        // Soft white→blue gradient backdrop, like the WLM sign-in window.
        container(body)
            .style(|_| container::Style {
                background: Some(MsnTheme::vertical_gradient(
                    iced::Color::WHITE,
                    MsnTheme::HEADER_BOTTOM,
                )),
                ..Default::default()
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
