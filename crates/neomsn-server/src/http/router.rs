use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;
use tracing::info;
use crate::state::SharedState;
use super::{auth, ws};

pub async fn run(state: SharedState) -> Result<()> {
    let app = Router::new()
        .route("/auth/signup", post(auth::signup))
        .route("/auth/login",  post(auth::login))
        .route("/ws",          get(ws::handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "0.0.0.0:8080";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("HTTP listening on {addr}");

    axum::serve(listener, app).await?;
    Ok(())
}
