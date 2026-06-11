use anyhow::{bail, Result};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc,
};
use tracing::debug;
use uuid::Uuid;
use neomsn_shared::proto::{Frame, FrameError};
use crate::state::{SessionHandle, SharedState};
use super::handlers::dispatch;

const READ_BUF: usize = 64 * 1024;
const CHANNEL_CAP: usize = 256;

pub async fn handle_connection(stream: TcpStream, state: SharedState) -> Result<()> {
    let (mut reader, mut writer) = stream.into_split();
    let (tx, mut rx) = mpsc::channel::<Frame>(CHANNEL_CAP);

    // Writer task: pulls frames from the channel and sends them on the wire.
    tokio::spawn(async move {
        while let Some(frame) = rx.recv().await {
            let bytes = frame.encode();
            if writer.write_all(&bytes).await.is_err() {
                break;
            }
        }
    });

    // Per-connection mutable state.
    let mut buf = Vec::with_capacity(READ_BUF);
    let mut session = SessionState::new(tx.clone());

    loop {
        let mut chunk = [0u8; READ_BUF];
        let n = reader.read(&mut chunk).await?;
        if n == 0 {
            break; // connection closed
        }
        buf.extend_from_slice(&chunk[..n]);

        // Drain as many complete frames as possible from the buffer.
        loop {
            match Frame::decode(&buf) {
                Ok((frame, consumed)) => {
                    buf.drain(..consumed);
                    debug!("← {:?}", frame.opcode);
                    dispatch(frame, &mut session, &state).await?;
                }
                Err(FrameError::Incomplete) => break,
                Err(e) => bail!("Frame error: {e}"),
            }
        }
    }

    // Clean up session if authenticated.
    if let Some((uid, did)) = session.identity() {
        state.remove_session(uid, did).await;
        super::handlers::handle_disconnect(uid, &state).await?;
    }

    Ok(())
}

/// Per-connection mutable state, managed by the read loop.
pub struct SessionState {
    pub tx: mpsc::Sender<Frame>,
    pub user_id: Option<Uuid>,
    pub device_id: Option<Uuid>,
}

impl SessionState {
    pub fn new(tx: mpsc::Sender<Frame>) -> Self {
        Self { tx, user_id: None, device_id: None }
    }

    pub fn is_authenticated(&self) -> bool {
        self.user_id.is_some()
    }

    pub fn identity(&self) -> Option<(Uuid, Uuid)> {
        match (self.user_id, self.device_id) {
            (Some(u), Some(d)) => Some((u, d)),
            _ => None,
        }
    }

    pub fn authenticate(&mut self, user_id: Uuid, device_id: Uuid) -> SessionHandle {
        self.user_id = Some(user_id);
        self.device_id = Some(device_id);
        SessionHandle { user_id, device_id, tx: self.tx.clone() }
    }

    pub async fn send(&self, frame: Frame) {
        let _ = self.tx.send(frame).await;
    }
}
