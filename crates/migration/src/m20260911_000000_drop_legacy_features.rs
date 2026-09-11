use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        
        // Drop legacy tables
        db.execute_unprepared("DROP TABLE IF EXISTS workflow").await?;
        db.execute_unprepared("DROP TABLE IF EXISTS saved_board_view").await?;
        db.execute_unprepared("DROP TABLE IF EXISTS entry_property_value").await?;
        db.execute_unprepared("DROP TABLE IF EXISTS entry_label").await?;
        db.execute_unprepared("DROP TABLE IF EXISTS entry_checklist_item").await?;
        db.execute_unprepared("DROP TABLE IF EXISTS entry_attachment").await?;
        db.execute_unprepared("DROP TABLE IF EXISTS entry").await?;
        db.execute_unprepared("DROP TABLE IF EXISTS card").await?;
        db.execute_unprepared("DROP TABLE IF EXISTS board_template").await?;
        db.execute_unprepared("DROP TABLE IF EXISTS board_property_option").await?;
        db.execute_unprepared("DROP TABLE IF EXISTS board_property").await?;
        db.execute_unprepared("DROP TABLE IF EXISTS board_label").await?;
        db.execute_unprepared("DROP TABLE IF EXISTS board").await?;
        db.execute_unprepared("DROP TABLE IF EXISTS board_old").await?;

        // Also clean up workspace_link
        // SQLite ALTER TABLE DROP COLUMN is available in recent versions, but maybe not in old ones.
        // It's safer to just let the columns sit there or drop them if supported.
        db.execute_unprepared("ALTER TABLE workspace_link DROP COLUMN target_board_id").await.ok();
        db.execute_unprepared("ALTER TABLE workspace_link DROP COLUMN target_card_id").await.ok();
        db.execute_unprepared("ALTER TABLE workspace_link DROP COLUMN target_entry_id").await.ok();
        db.execute_unprepared("ALTER TABLE workspace_link DROP COLUMN source_entry_id").await.ok();
        db.execute_unprepared("ALTER TABLE workspace_link DROP COLUMN target_saved_view_id").await.ok();
        
        db.execute_unprepared("ALTER TABLE workspace_reference_alias DROP COLUMN board_id").await.ok();
        db.execute_unprepared("ALTER TABLE workspace_reference_alias DROP COLUMN list_id").await.ok();
        db.execute_unprepared("ALTER TABLE workspace_reference_alias DROP COLUMN card_id").await.ok();
        db.execute_unprepared("ALTER TABLE workspace_reference_alias DROP COLUMN saved_view_id").await.ok();
        
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
