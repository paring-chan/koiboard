use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        manager
            .create_table(
                Table::create()
                    .table(Koi::Table)
                    .col(pk_auto(Koi::Id))
                    .col(string_uniq(Koi::ReferenceId))
                    .col(string_uniq(Koi::CounterId))
                    .take(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Koi::Table).take())
            .await
    }
}

#[derive(DeriveIden)]
enum Koi {
    Table,
    Id,
    ReferenceId,
    CounterId,
}
