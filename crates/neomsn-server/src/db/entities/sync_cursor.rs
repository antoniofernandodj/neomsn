use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sync_cursors")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub device_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub context_id: Uuid,
    /// Last `message_chunks.id` received by this device in this context.
    pub last_chunk_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
