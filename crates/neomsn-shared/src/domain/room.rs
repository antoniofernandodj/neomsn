use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Room {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub member_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomRole { Member, Admin, Owner }

#[derive(Debug, Clone)]
pub struct RoomMember {
    pub user_id: Uuid,
    pub display_name: String,
    pub role: RoomRole,
}
