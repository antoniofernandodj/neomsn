use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "message_chunks")]
pub struct Model {
    /// Global monotonic id — used as the sync cursor value.
    #[sea_orm(primary_key)]
    pub id: i64,
    pub message_id: Uuid,
    pub delta: String,
    /// Position within the message (0-based).
    pub seq: i32,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
