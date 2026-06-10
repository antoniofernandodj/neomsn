use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "room_members")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub room_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: Uuid,
    /// "member" | "admin" | "owner"
    pub role: String,
    pub joined_at: DateTimeUtc,
    pub left_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
