mod auth;
mod db;
mod http;
mod nmp;
mod state;

use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db = db::connect("neomsn.db").await?;
    let state = state::AppState::new(db);

    tokio::try_join!(
        nmp::listener::run(state.clone()),
        http::router::run(state.clone()),
    )?;

    Ok(())
}
