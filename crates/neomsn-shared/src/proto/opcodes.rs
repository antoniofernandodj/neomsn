/// Every opcode that can appear in an NMP frame.
/// Grouped by range — see CLAUDE.md for the full table.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    // 0x01–0x0F  Session / Handshake
    Hello       = 0x01,
    Auth        = 0x02,
    AuthOk      = 0x03,
    AuthFail    = 0x04,

    // 0x10–0x1F  Messages (streaming)
    MsgChunk    = 0x10,
    MsgComplete = 0x11,
    MsgDelete   = 0x12,

    // 0x20–0x2F  Rooms
    RoomCreate   = 0x20,
    RoomCreateOk = 0x21,
    RoomUpdate   = 0x22,
    RoomDelete   = 0x23,
    RoomJoin     = 0x24,
    RoomLeave    = 0x25,
    RoomList     = 0x26,
    RoomListResp = 0x27,
    RoomMembers  = 0x28,
    RoomEvent    = 0x29,

    // 0x30–0x3F  Direct Messages
    DmOpen     = 0x30,
    DmOpenResp = 0x31,

    // 0x40–0x4F  Presence
    PresenceSet    = 0x40,
    PresenceUpdate = 0x41,

    // 0x50–0x5F  Sync
    SyncRequest  = 0x50,
    SyncResponse = 0x51,

    // 0x60–0x6F  Profile
    ProfileGet      = 0x60,
    ProfileResp     = 0x61,
    ProfileUpdate   = 0x62,
    ProfileUpdateOk = 0x63,

    // 0x70–0x7F  Contacts
    ContactList     = 0x70,
    ContactListResp = 0x71,
    ContactAdd      = 0x72,
    ContactAddOk    = 0x73,
    ContactRemove   = 0x74,
    ContactBlock    = 0x75,
    ContactUnblock  = 0x76,
    ContactRequest  = 0x77,
    ContactAccept   = 0x78,
    ContactAcceptOk = 0x79,
    ContactReject   = 0x7A,

    // 0xF0–0xFF  System
    Error = 0xF0,
    Ping  = 0xFE,
    Pong  = 0xFF,
}

impl TryFrom<u8> for Opcode {
    type Error = u8;

    fn try_from(v: u8) -> Result<Self, <Self as TryFrom<u8>>::Error> {
        match v {
            0x01 => Ok(Self::Hello),
            0x02 => Ok(Self::Auth),
            0x03 => Ok(Self::AuthOk),
            0x04 => Ok(Self::AuthFail),
            0x10 => Ok(Self::MsgChunk),
            0x11 => Ok(Self::MsgComplete),
            0x12 => Ok(Self::MsgDelete),
            0x20 => Ok(Self::RoomCreate),
            0x21 => Ok(Self::RoomCreateOk),
            0x22 => Ok(Self::RoomUpdate),
            0x23 => Ok(Self::RoomDelete),
            0x24 => Ok(Self::RoomJoin),
            0x25 => Ok(Self::RoomLeave),
            0x26 => Ok(Self::RoomList),
            0x27 => Ok(Self::RoomListResp),
            0x28 => Ok(Self::RoomMembers),
            0x29 => Ok(Self::RoomEvent),
            0x30 => Ok(Self::DmOpen),
            0x31 => Ok(Self::DmOpenResp),
            0x40 => Ok(Self::PresenceSet),
            0x41 => Ok(Self::PresenceUpdate),
            0x50 => Ok(Self::SyncRequest),
            0x51 => Ok(Self::SyncResponse),
            0x60 => Ok(Self::ProfileGet),
            0x61 => Ok(Self::ProfileResp),
            0x62 => Ok(Self::ProfileUpdate),
            0x63 => Ok(Self::ProfileUpdateOk),
            0x70 => Ok(Self::ContactList),
            0x71 => Ok(Self::ContactListResp),
            0x72 => Ok(Self::ContactAdd),
            0x73 => Ok(Self::ContactAddOk),
            0x74 => Ok(Self::ContactRemove),
            0x75 => Ok(Self::ContactBlock),
            0x76 => Ok(Self::ContactUnblock),
            0x77 => Ok(Self::ContactRequest),
            0x78 => Ok(Self::ContactAccept),
            0x79 => Ok(Self::ContactAcceptOk),
            0x7A => Ok(Self::ContactReject),
            0xF0 => Ok(Self::Error),
            0xFE => Ok(Self::Ping),
            0xFF => Ok(Self::Pong),
            other => Err(other),
        }
    }
}
