use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "contacts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub owner_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub contact_id: Uuid,
    /// "pending" | "accepted" | "blocked"
    pub state: String,
    pub since: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
