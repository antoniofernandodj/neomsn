use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub personal_message: String,
    pub avatar_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceStatus {
    Online,
    Away,
    Busy,
    Invisible,
    Offline,
}

impl PresenceStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Online    => "Online",
            Self::Away      => "Ausente",
            Self::Busy      => "Ocupado",
            Self::Invisible => "Invisível",
            Self::Offline   => "Offline",
        }
    }
}

impl From<crate::proto::payload::PresenceStatus> for PresenceStatus {
    fn from(p: crate::proto::payload::PresenceStatus) -> Self {
        use crate::proto::payload::PresenceStatus as P;
        match p {
            P::Online    => Self::Online,
            P::Away      => Self::Away,
            P::Busy      => Self::Busy,
            P::Invisible => Self::Invisible,
            P::Offline   => Self::Offline,
        }
    }
}

impl From<PresenceStatus> for crate::proto::payload::PresenceStatus {
    fn from(p: PresenceStatus) -> Self {
        use crate::proto::payload::PresenceStatus as P;
        match p {
            PresenceStatus::Online    => P::Online,
            PresenceStatus::Away      => P::Away,
            PresenceStatus::Busy      => P::Busy,
            PresenceStatus::Invisible => P::Invisible,
            PresenceStatus::Offline   => P::Offline,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Presence {
    pub user_id: Uuid,
    pub status: PresenceStatus,
}
