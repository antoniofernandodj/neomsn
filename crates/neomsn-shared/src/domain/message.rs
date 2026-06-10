use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageStatus {
    Streaming,
    Complete,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextType {
    Room,
    Dm,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id: Uuid,
    pub context_type: ContextType,
    pub context_id: Uuid,
    pub author_id: Uuid,
    pub author_display_name: String,
    pub content: String,
    pub status: MessageStatus,
}

impl Message {
    pub fn new_streaming(
        id: Uuid,
        context_type: ContextType,
        context_id: Uuid,
        author_id: Uuid,
        author_display_name: String,
    ) -> Self {
        Self {
            id,
            context_type,
            context_id,
            author_id,
            author_display_name,
            content: String::new(),
            status: MessageStatus::Streaming,
        }
    }

    pub fn apply_chunk(&mut self, delta: &str) {
        self.content.push_str(delta);
    }

    pub fn complete(&mut self) {
        self.status = MessageStatus::Complete;
    }

    pub fn delete(&mut self) {
        self.status = MessageStatus::Deleted;
    }
}
