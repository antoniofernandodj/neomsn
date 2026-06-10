use iced::{
    widget::{button, column, container, row, text, text_input, Space},
    Alignment, Element, Length, Padding,
};
use neomsn_shared::widgets::theme::MsnTheme;

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

    pub fn view(&self) -> Element<LoginMsg> {
        let title = text(if self.signup_mode { "Criar conta" } else { "NeoMSN" })
            .size(28)
            .color(MsnTheme::TEXT_ON_ACCENT);

        let header = container(title)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(MsnTheme::HEADER_TOP)),
                ..Default::default()
            })
            .padding(Padding::from([20, 30]))
            .width(Length::Fill);

        let username_field = text_input("Usuário", &self.username)
            .on_input(LoginMsg::UsernameChanged)
            .padding(8);

        let password_field = text_input("Senha", &self.password)
            .on_input(LoginMsg::PasswordChanged)
            .secure(true)
            .padding(8);

        let mut form = column![
            text("Usuário").size(13).color(MsnTheme::TEXT_SECONDARY),
            username_field,
            Space::with_height(8),
            text("Senha").size(13).color(MsnTheme::TEXT_SECONDARY),
            password_field,
        ]
        .spacing(4);

        if self.signup_mode {
            let dn_field = text_input("Nome de exibição", &self.display_name)
                .on_input(LoginMsg::DisplayNameChanged)
                .padding(8);
            form = form
                .push(Space::with_height(8))
                .push(text("Nome de exibição").size(13).color(MsnTheme::TEXT_SECONDARY))
                .push(dn_field);
        }

        let submit_label = if self.loading { "Aguarde…" } else if self.signup_mode { "Criar conta" } else { "Entrar" };
        let mut submit_btn = button(text(submit_label).size(14)).padding(Padding::from([8, 20]));
        if !self.loading {
            submit_btn = submit_btn.on_press(LoginMsg::Submit);
        }

        let toggle_label = if self.signup_mode { "Já tenho conta" } else { "Criar conta" };
        let toggle_btn = button(text(toggle_label).size(12).color(MsnTheme::ACCENT))
            .style(button::text)
            .on_press(LoginMsg::ToggleSignup);

        let error_row: Element<LoginMsg> = if let Some(e) = &self.error {
            text(e).size(12).color(MsnTheme::BUSY).into()
        } else {
            Space::with_height(0).into()
        };

        let actions = row![submit_btn, Space::with_width(Length::Fill), toggle_btn]
            .align_y(Alignment::Center);

        let body = container(
            column![form, Space::with_height(16), error_row, Space::with_height(8), actions]
                .spacing(0),
        )
        .padding(Padding::from([30, 30]))
        .max_width(380)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(MsnTheme::BG_PANEL)),
            border: iced::Border { radius: 0.0.into(), ..Default::default() },
            ..Default::default()
        });

        column![header, body].into()
    }
}
