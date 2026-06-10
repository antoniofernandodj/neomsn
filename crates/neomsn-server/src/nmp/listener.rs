use anyhow::Result;
use tokio::net::TcpListener;
use tracing::{error, info};
use crate::state::SharedState;
use super::session::handle_connection;

pub async fn run(state: SharedState) -> Result<()> {
    let addr = "0.0.0.0:7777";
    let listener = TcpListener::bind(addr).await?;
    info!("NMP listening on {addr}");

    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                info!("NMP connection from {peer}");
                let s = state.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, s).await {
                        error!("NMP session error: {e}");
                    }
                });
            }
            Err(e) => error!("Accept error: {e}"),
        }
    }
}
