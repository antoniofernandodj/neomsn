use anyhow::Result;
use serde::{Deserialize, Serialize};

const BASE: &str = "http://127.0.0.1:8080";

#[derive(Clone)]
pub struct HttpClient {
    inner: reqwest::Client,
}

impl HttpClient {
    pub fn new() -> Self {
        Self { inner: reqwest::Client::new() }
    }

    pub async fn signup(
        &self,
        username: &str,
        password: &str,
        display_name: &str,
    ) -> Result<AuthResponse> {
        let resp = self.inner
            .post(format!("{BASE}/auth/signup"))
            .json(&SignupRequest {
                username: username.into(),
                password: password.into(),
                display_name: display_name.into(),
                device_name: hostname(),
            })
            .send().await?
            .error_for_status()?
            .json::<AuthResponse>().await?;
        Ok(resp)
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<AuthResponse> {
        let resp = self.inner
            .post(format!("{BASE}/auth/login"))
            .json(&LoginRequest {
                username: username.into(),
                password: password.into(),
                device_name: hostname(),
            })
            .send().await?
            .error_for_status()?
            .json::<AuthResponse>().await?;
        Ok(resp)
    }
}

fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "Desktop".into())
}

#[derive(Serialize)]
struct SignupRequest {
    username: String,
    password: String,
    display_name: String,
    device_name: String,
}

#[derive(Serialize)]
struct LoginRequest {
    username: String,
    password: String,
    device_name: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct AuthResponse {
    pub token: String,
    pub device_id: String,
    pub user_id: String,
}
