use axum::{
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
};
use axum::extract::ws::{Message, WebSocket};
use tracing::error;
use neomsn_shared::proto::{Frame, FrameError};
use crate::{nmp::session::SessionState, state::SharedState};
use tokio::sync::mpsc;
use crate::nmp::handlers::dispatch;

pub async fn handler(
    ws: WebSocketUpgrade,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_ws(socket, state))
}

async fn handle_ws(socket: WebSocket, state: SharedState) {
    if let Err(e) = run_ws(socket, state).await {
        error!("WS session error: {e}");
    }
}

async fn run_ws(mut socket: WebSocket, state: SharedState) -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::channel::<Frame>(256);
    let mut session = SessionState::new(tx.clone());
    let mut buf = Vec::<u8>::new();

    loop {
        tokio::select! {
            // Outbound: NMP frame → binary WS message.
            Some(frame) = rx.recv() => {
                let bytes = frame.encode();
                if socket.send(Message::Binary(bytes.into())).await.is_err() {
                    break;
                }
            }
            // Inbound: binary WS message → NMP frame dispatch.
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Binary(data))) => {
                        buf.extend_from_slice(&data);
                        loop {
                            match Frame::decode(&buf) {
                                Ok((frame, consumed)) => {
                                    buf.drain(..consumed);
                                    dispatch(frame, &mut session, &state).await?;
                                }
                                Err(FrameError::Incomplete) => break,
                                Err(e) => return Err(anyhow::anyhow!("{e}")),
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(e)) => return Err(e.into()),
                }
            }
        }
    }

    if let Some((uid, did)) = session.identity() {
        state.remove_session(uid, did).await;
    }
    Ok(())
}
