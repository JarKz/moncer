use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table("category")
                    .if_not_exists()
                    .col(pk_auto("id"))
                    .col(string("name"))
                    .col(integer_null("parent_category_id"))
                    .foreign_key(
                        ForeignKey::create()
                            .name("parent_category_fk")
                            .from("category", "parent_category_id")
                            .to("category", "id")
                            .on_delete(ForeignKeyAction::NoAction)
                            .on_update(ForeignKeyAction::NoAction),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("uk_category_parent_name")
                    .table("category")
                    .col("parent_category_id")
                    .col("name")
                    .and_where(Expr::col(("category", "parent_category_id")).is_not_null())
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("uk_category_root_name")
                    .table("category")
                    .col("name")
                    .and_where(Expr::col(("category", "parent_category_id")).is_null())
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table("transaction")
                    .if_not_exists()
                    .col(pk_auto("id"))
                    .col(integer("change_minor"))
                    .col(integer_null("category_id"))
                    .col(string_null("reason"))
                    .col(date_time("created_at"))
                    .foreign_key(
                        ForeignKey::create()
                            .name("transaction_fk")
                            .from("transaction", "category_id")
                            .to("category", "id")
                            .on_delete(ForeignKeyAction::NoAction)
                            .on_update(ForeignKeyAction::NoAction),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table("transaction").to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("uk_category_parent_name").to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("uk_category_root_name").to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table("category").to_owned())
            .await
    }
}
