use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(M20240101CreateTables)]
    }
}

struct M20240101CreateTables;

impl MigrationName for M20240101CreateTables {
    fn name(&self) -> &str { "m20240101_000001_create_tables" }
}

#[async_trait::async_trait]
impl MigrationTrait for M20240101CreateTables {
    async fn up(&self, mgr: &SchemaManager) -> Result<(), DbErr> {
        mgr.create_table(
            Table::create().table(Users::Table).if_not_exists()
                .col(ColumnDef::new(Users::Id).uuid().not_null().primary_key())
                .col(ColumnDef::new(Users::Username).string().not_null().unique_key())
                .col(ColumnDef::new(Users::DisplayName).string().not_null())
                .col(ColumnDef::new(Users::PersonalMessage).string().not_null().default(""))
                .col(ColumnDef::new(Users::AvatarUrl).string().not_null().default(""))
                .col(ColumnDef::new(Users::PasswordHash).string().not_null())
                .col(ColumnDef::new(Users::CreatedAt).timestamp_with_time_zone().not_null())
                .col(ColumnDef::new(Users::DeletedAt).timestamp_with_time_zone().null())
                .to_owned(),
        ).await?;

        mgr.create_table(
            Table::create().table(Devices::Table).if_not_exists()
                .col(ColumnDef::new(Devices::Id).uuid().not_null().primary_key())
                .col(ColumnDef::new(Devices::UserId).uuid().not_null())
                .col(ColumnDef::new(Devices::Name).string().not_null())
                .col(ColumnDef::new(Devices::Platform).string().not_null())
                .col(ColumnDef::new(Devices::LastSeenAt).timestamp_with_time_zone().not_null())
                .to_owned(),
        ).await?;

        mgr.create_table(
            Table::create().table(Contacts::Table).if_not_exists()
                .col(ColumnDef::new(Contacts::OwnerId).uuid().not_null())
                .col(ColumnDef::new(Contacts::ContactId).uuid().not_null())
                .col(ColumnDef::new(Contacts::State).string().not_null())
                .col(ColumnDef::new(Contacts::Since).timestamp_with_time_zone().not_null())
                .primary_key(Index::create().col(Contacts::OwnerId).col(Contacts::ContactId))
                .to_owned(),
        ).await?;

        mgr.create_table(
            Table::create().table(Rooms::Table).if_not_exists()
                .col(ColumnDef::new(Rooms::Id).uuid().not_null().primary_key())
                .col(ColumnDef::new(Rooms::Name).string().not_null())
                .col(ColumnDef::new(Rooms::Description).string().not_null().default(""))
                .col(ColumnDef::new(Rooms::CreatedBy).uuid().not_null())
                .col(ColumnDef::new(Rooms::CreatedAt).timestamp_with_time_zone().not_null())
                .col(ColumnDef::new(Rooms::DeletedAt).timestamp_with_time_zone().null())
                .to_owned(),
        ).await?;

        mgr.create_table(
            Table::create().table(RoomMembers::Table).if_not_exists()
                .col(ColumnDef::new(RoomMembers::RoomId).uuid().not_null())
                .col(ColumnDef::new(RoomMembers::UserId).uuid().not_null())
                .col(ColumnDef::new(RoomMembers::Role).string().not_null().default("member"))
                .col(ColumnDef::new(RoomMembers::JoinedAt).timestamp_with_time_zone().not_null())
                .col(ColumnDef::new(RoomMembers::LeftAt).timestamp_with_time_zone().null())
                .primary_key(Index::create().col(RoomMembers::RoomId).col(RoomMembers::UserId))
                .to_owned(),
        ).await?;

        mgr.create_table(
            Table::create().table(DirectConversations::Table).if_not_exists()
                .col(ColumnDef::new(DirectConversations::Id).uuid().not_null().primary_key())
                .col(ColumnDef::new(DirectConversations::UserA).uuid().not_null())
                .col(ColumnDef::new(DirectConversations::UserB).uuid().not_null())
                .col(ColumnDef::new(DirectConversations::CreatedAt).timestamp_with_time_zone().not_null())
                .to_owned(),
        ).await?;

        mgr.create_table(
            Table::create().table(Messages::Table).if_not_exists()
                .col(ColumnDef::new(Messages::Id).uuid().not_null().primary_key())
                .col(ColumnDef::new(Messages::ContextType).string().not_null())
                .col(ColumnDef::new(Messages::ContextId).uuid().not_null())
                .col(ColumnDef::new(Messages::AuthorId).uuid().not_null())
                .col(ColumnDef::new(Messages::Content).text().not_null().default(""))
                .col(ColumnDef::new(Messages::Status).string().not_null().default("streaming"))
                .col(ColumnDef::new(Messages::StartedAt).timestamp_with_time_zone().not_null())
                .col(ColumnDef::new(Messages::CompletedAt).timestamp_with_time_zone().null())
                .to_owned(),
        ).await?;

        mgr.create_table(
            Table::create().table(MessageChunks::Table).if_not_exists()
                .col(ColumnDef::new(MessageChunks::Id).big_integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(MessageChunks::MessageId).uuid().not_null())
                .col(ColumnDef::new(MessageChunks::Delta).string().not_null())
                .col(ColumnDef::new(MessageChunks::Seq).integer().not_null())
                .col(ColumnDef::new(MessageChunks::CreatedAt).timestamp_with_time_zone().not_null())
                .to_owned(),
        ).await?;

        mgr.create_table(
            Table::create().table(SyncCursors::Table).if_not_exists()
                .col(ColumnDef::new(SyncCursors::DeviceId).uuid().not_null())
                .col(ColumnDef::new(SyncCursors::ContextId).uuid().not_null())
                .col(ColumnDef::new(SyncCursors::LastChunkId).big_integer().not_null().default(0i64))
                .primary_key(Index::create().col(SyncCursors::DeviceId).col(SyncCursors::ContextId))
                .to_owned(),
        ).await?;

        Ok(())
    }

    async fn down(&self, mgr: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            "sync_cursors", "message_chunks", "messages",
            "direct_conversations", "room_members", "rooms",
            "contacts", "devices", "users",
        ] {
            mgr.get_connection()
                .execute_unprepared(&format!("DROP TABLE IF EXISTS {table}"))
                .await?;
        }
        Ok(())
    }
}

// ─── Iden enums ──────────────────────────────────────────────────────────────

#[derive(Iden)] enum Users { Table, Id, Username, DisplayName, PersonalMessage, AvatarUrl, PasswordHash, CreatedAt, DeletedAt }
#[derive(Iden)] enum Devices { Table, Id, UserId, Name, Platform, LastSeenAt }
#[derive(Iden)] enum Contacts { Table, OwnerId, ContactId, State, Since }
#[derive(Iden)] enum Rooms { Table, Id, Name, Description, CreatedBy, CreatedAt, DeletedAt }
#[derive(Iden)] enum RoomMembers { Table, RoomId, UserId, Role, JoinedAt, LeftAt }
#[derive(Iden)] enum DirectConversations { Table, Id, UserA, UserB, CreatedAt }
#[derive(Iden)] enum Messages { Table, Id, ContextType, ContextId, AuthorId, Content, Status, StartedAt, CompletedAt }
#[derive(Iden)] enum MessageChunks { Table, Id, MessageId, Delta, Seq, CreatedAt }
#[derive(Iden)] enum SyncCursors { Table, DeviceId, ContextId, LastChunkId }
