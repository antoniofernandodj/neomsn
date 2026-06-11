use anyhow::Result;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc,
};
use neomsn_shared::proto::{Frame, FrameError, Opcode, payload};
use uuid::Uuid;

const NMP_ADDR: &str = "127.0.0.1:7777";
const CHANNEL_CAP: usize = 256;

/// Handle owned by the app for sending frames to the server.
#[derive(Clone, Debug)]
pub struct NmpClient {
    pub tx: mpsc::Sender<Frame>,
}

impl NmpClient {
    pub fn send(&self, frame: Frame) {
        let _ = self.tx.try_send(frame);
    }
}

/// Event received from the server, forwarded to the Iced message loop.
#[derive(Debug, Clone)]
pub enum ServerEvent {
    AuthOk(payload::AuthOk),
    AuthFail(payload::AuthFail),
    MsgChunk(payload::MsgChunk),
    MsgComplete(payload::MsgComplete),
    MsgDelete(payload::MsgDelete),
    Nudge(payload::Nudge),
    ContactListResp(payload::ContactListResp),
    ContactAddOk(payload::ContactAddOk),
    ContactRequest(payload::ContactRequest),
    ContactAcceptOk(payload::ContactAcceptOk),
    DmOpenResp(payload::DmOpenResp),
    PresenceUpdate(payload::PresenceUpdate),
    RoomListResp(payload::RoomListResp),
    SyncResponse(payload::SyncResponse),
    ChatJoined(payload::ChatJoined),
    RoomEvent(payload::RoomEvent),
    ProfileResp(payload::ProfileResp),
    Error(payload::Error),
    Disconnected,
}

/// Connect to the NMP server and start the read/write loops.
/// Returns a `NmpClient` for sending and an `mpsc::Receiver` for receiving events.
pub async fn connect(
    token: String,
    device_id: Uuid,
) -> Result<(NmpClient, mpsc::Receiver<ServerEvent>)> {
    let stream = TcpStream::connect(NMP_ADDR).await?;
    let (mut reader, mut writer) = stream.into_split();

    let (out_tx, mut out_rx) = mpsc::channel::<Frame>(CHANNEL_CAP);
    let (in_tx, in_rx) = mpsc::channel::<ServerEvent>(CHANNEL_CAP);

    // Writer task.
    tokio::spawn(async move {
        while let Some(frame) = out_rx.recv().await {
            if writer.write_all(&frame.encode()).await.is_err() { break; }
        }
    });

    let client = NmpClient { tx: out_tx.clone() };

    // Send HELLO + AUTH immediately.
    let hello = Frame::new(Opcode::Hello, payload::Hello {
        proto_version: 1,
        device_id,
    }.encode());
    let auth = Frame::new(Opcode::Auth, payload::Auth { token }.encode());
    out_tx.send(hello).await?;
    out_tx.send(auth).await?;

    // Reader task.
    tokio::spawn(async move {
        let mut buf = Vec::<u8>::new();
        let mut tmp = [0u8; 65536];

        loop {
            match reader.read(&mut tmp).await {
                Ok(0) | Err(_) => {
                    let _ = in_tx.send(ServerEvent::Disconnected).await;
                    break;
                }
                Ok(n) => buf.extend_from_slice(&tmp[..n]),
            }

            loop {
                match Frame::decode(&buf) {
                    Ok((frame, consumed)) => {
                        buf.drain(..consumed);
                        if let Some(event) = decode_server_frame(frame) {
                            if in_tx.send(event).await.is_err() { return; }
                        }
                    }
                    Err(FrameError::Incomplete) => break,
                    Err(_) => return,
                }
            }
        }
    });

    Ok((client, in_rx))
}

fn decode_server_frame(frame: Frame) -> Option<ServerEvent> {
    let p = &frame.payload;
    match frame.opcode {
        Opcode::AuthOk          => payload::AuthOk::decode(p).ok().map(ServerEvent::AuthOk),
        Opcode::AuthFail        => payload::AuthFail::decode(p).ok().map(ServerEvent::AuthFail),
        Opcode::MsgChunk        => payload::MsgChunk::decode(p).ok().map(ServerEvent::MsgChunk),
        Opcode::MsgComplete     => payload::MsgComplete::decode(p).ok().map(ServerEvent::MsgComplete),
        Opcode::MsgDelete       => payload::MsgDelete::decode(p).ok().map(ServerEvent::MsgDelete),
        Opcode::Nudge           => payload::Nudge::decode(p).ok().map(ServerEvent::Nudge),
        Opcode::ContactListResp => payload::ContactListResp::decode(p).ok().map(ServerEvent::ContactListResp),
        Opcode::ContactAddOk    => payload::ContactAddOk::decode(p).ok().map(ServerEvent::ContactAddOk),
        Opcode::ContactRequest  => payload::ContactRequest::decode(p).ok().map(ServerEvent::ContactRequest),
        Opcode::ContactAcceptOk => payload::ContactAcceptOk::decode(p).ok().map(ServerEvent::ContactAcceptOk),
        Opcode::DmOpenResp      => payload::DmOpenResp::decode(p).ok().map(ServerEvent::DmOpenResp),
        Opcode::PresenceUpdate  => payload::PresenceUpdate::decode(p).ok().map(ServerEvent::PresenceUpdate),
        Opcode::RoomListResp    => payload::RoomListResp::decode(p).ok().map(ServerEvent::RoomListResp),
        Opcode::SyncResponse    => payload::SyncResponse::decode(p).ok().map(ServerEvent::SyncResponse),
        Opcode::ChatJoined      => payload::ChatJoined::decode(p).ok().map(ServerEvent::ChatJoined),
        Opcode::RoomEvent       => payload::RoomEvent::decode(p).ok().map(ServerEvent::RoomEvent),
        Opcode::ProfileResp     => payload::ProfileResp::decode(p).ok().map(ServerEvent::ProfileResp),
        Opcode::Error           => payload::Error::decode(p).ok().map(ServerEvent::Error),
        _ => None,
    }
}
