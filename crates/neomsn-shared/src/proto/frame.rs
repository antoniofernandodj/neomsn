use super::Opcode;

pub const MAGIC: [u8; 2] = [0x4E, 0x4D]; // "NM"
pub const HEADER_LEN: usize = 7; // magic(2) + opcode(1) + length(4)
pub const MAX_PAYLOAD: usize = 16 * 1024 * 1024; // 16 MiB safety cap

#[derive(Debug, Clone)]
pub struct Frame {
    pub opcode: Opcode,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub enum FrameError {
    /// Not enough bytes yet — caller should buffer and retry.
    Incomplete,
    InvalidMagic,
    UnknownOpcode(u8),
    PayloadTooLarge(usize),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Incomplete => write!(f, "incomplete frame"),
            Self::InvalidMagic => write!(f, "invalid magic bytes"),
            Self::UnknownOpcode(b) => write!(f, "unknown opcode 0x{b:02X}"),
            Self::PayloadTooLarge(n) => write!(f, "payload too large: {n} bytes"),
        }
    }
}

impl std::error::Error for FrameError {}

impl Frame {
    pub fn new(opcode: Opcode, payload: Vec<u8>) -> Self {
        Self { opcode, payload }
    }

    /// Encode into wire bytes.
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(HEADER_LEN + self.payload.len());
        buf.extend_from_slice(&MAGIC);
        buf.push(self.opcode as u8);
        buf.extend_from_slice(&(self.payload.len() as u32).to_be_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    /// Try to decode one frame from a byte slice.
    /// Returns `(frame, bytes_consumed)` on success.
    pub fn decode(buf: &[u8]) -> Result<(Frame, usize), FrameError> {
        if buf.len() < HEADER_LEN {
            return Err(FrameError::Incomplete);
        }
        if buf[..2] != MAGIC {
            return Err(FrameError::InvalidMagic);
        }
        let opcode = Opcode::try_from(buf[2]).map_err(FrameError::UnknownOpcode)?;
        let length = u32::from_be_bytes([buf[3], buf[4], buf[5], buf[6]]) as usize;
        if length > MAX_PAYLOAD {
            return Err(FrameError::PayloadTooLarge(length));
        }
        let total = HEADER_LEN + length;
        if buf.len() < total {
            return Err(FrameError::Incomplete);
        }
        let payload = buf[HEADER_LEN..total].to_vec();
        Ok((Frame { opcode, payload }, total))
    }
}
