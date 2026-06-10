use std::{collections::{HashMap, HashSet}, sync::Arc};
use sea_orm::DatabaseConnection;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use neomsn_shared::proto::Frame;
use neomsn_shared::proto::payload::PresenceStatus;

pub type SharedState = Arc<AppState>;

pub struct AppState {
    pub db: DatabaseConnection,
    /// user_id → active session (may be multiple devices, but one sender per TCP conn)
    pub sessions: RwLock<HashMap<Uuid, Vec<SessionHandle>>>,
    /// user_id → last known presence
    pub presence: RwLock<HashMap<Uuid, PresenceStatus>>,
    /// room_id → set of user_ids currently in the room
    pub room_members: RwLock<HashMap<Uuid, HashSet<Uuid>>>,
}

#[derive(Clone)]
pub struct SessionHandle {
    pub user_id: Uuid,
    pub device_id: Uuid,
    pub tx: mpsc::Sender<Frame>,
}

impl AppState {
    pub fn new(db: DatabaseConnection) -> Arc<Self> {
        Arc::new(Self {
            db,
            sessions: RwLock::new(HashMap::new()),
            presence: RwLock::new(HashMap::new()),
            room_members: RwLock::new(HashMap::new()),
        })
    }

    /// Register a newly authenticated session.
    pub async fn add_session(&self, handle: SessionHandle) {
        self.sessions
            .write().await
            .entry(handle.user_id)
            .or_default()
            .push(handle);
    }

    /// Remove a session when its TCP connection drops.
    pub async fn remove_session(&self, user_id: Uuid, device_id: Uuid) {
        let mut sessions = self.sessions.write().await;
        if let Some(list) = sessions.get_mut(&user_id) {
            list.retain(|h| h.device_id != device_id);
            if list.is_empty() {
                sessions.remove(&user_id);
            }
        }
    }

    /// Send a frame to every active session of a user.
    pub async fn send_to_user(&self, user_id: Uuid, frame: Frame) {
        let sessions = self.sessions.read().await;
        if let Some(handles) = sessions.get(&user_id) {
            for h in handles {
                let _ = h.tx.try_send(frame.clone());
            }
        }
    }

    /// Broadcast a frame to everyone in a room (excluding the sender).
    pub async fn broadcast_room(&self, room_id: Uuid, frame: Frame, except: Option<Uuid>) {
        let members = {
            let rm = self.room_members.read().await;
            rm.get(&room_id).cloned().unwrap_or_default()
        };
        for uid in members {
            if Some(uid) == except { continue; }
            self.send_to_user(uid, frame.clone()).await;
        }
    }

    /// Send a frame to exactly the two participants of a DM.
    pub async fn broadcast_dm(&self, user_a: Uuid, user_b: Uuid, frame: Frame, except: Option<Uuid>) {
        for uid in [user_a, user_b] {
            if Some(uid) == except { continue; }
            self.send_to_user(uid, frame.clone()).await;
        }
    }
}
