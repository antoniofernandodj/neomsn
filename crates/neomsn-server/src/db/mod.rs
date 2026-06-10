pub mod entities;
pub mod migration;

use anyhow::Result;
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;

pub async fn connect(path: &str) -> Result<DatabaseConnection> {
    let url = format!("sqlite://{path}?mode=rwc");
    let db = Database::connect(&url).await?;
    migration::Migrator::up(&db, None).await?;
    Ok(db)
}
