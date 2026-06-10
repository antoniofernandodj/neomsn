use uuid::Uuid;
use super::user::PresenceStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactState { Pending, Accepted, Blocked }

#[derive(Debug, Clone)]
pub struct Contact {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub presence: PresenceStatus,
    pub state: ContactState,
}
