use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"
            CREATE TABLE IF NOT EXISTS project (
                id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                folder_path TEXT NULL,
                archived BOOLEAN NOT NULL DEFAULT 0,
                position INTEGER NOT NULL DEFAULT 0,
                deleted_at INTEGER NULL
            );
            CREATE INDEX IF NOT EXISTS idx_project_deleted_at ON project(deleted_at);

            CREATE TABLE IF NOT EXISTS note (
                id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                project_id INTEGER NULL,
                file_path TEXT NULL,
                file_managed_by_app BOOLEAN NOT NULL DEFAULT 1,
                cached_content TEXT NOT NULL DEFAULT '',
                file_missing_since INTEGER NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                is_pinned BOOLEAN NOT NULL DEFAULT 0,
                last_opened_at INTEGER NULL,
                deleted_at INTEGER NULL,
                FOREIGN KEY (project_id) REFERENCES project(id) ON DELETE SET NULL
            );
            CREATE INDEX IF NOT EXISTS idx_note_project_id ON note(project_id);
            CREATE INDEX IF NOT EXISTS idx_note_deleted_at ON note(deleted_at);
            CREATE INDEX IF NOT EXISTS idx_note_is_pinned ON note(is_pinned);
            CREATE INDEX IF NOT EXISTS idx_note_updated_at ON note(updated_at);

            CREATE TABLE IF NOT EXISTS note_alias (
                id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                note_id INTEGER NOT NULL,
                alias TEXT NOT NULL,
                normalized_alias TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (note_id) REFERENCES note(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_note_alias_normalized ON note_alias(normalized_alias);
            CREATE INDEX IF NOT EXISTS idx_note_alias_note_id ON note_alias(note_id);

            CREATE TABLE IF NOT EXISTS note_link (
                source_note_id INTEGER NOT NULL,
                ordinal INTEGER NOT NULL,
                target_note_id INTEGER NULL,
                raw_target TEXT NOT NULL,
                display_text TEXT NULL,
                start_byte INTEGER NOT NULL,
                end_byte INTEGER NOT NULL,
                line_number INTEGER NOT NULL,
                PRIMARY KEY (source_note_id, ordinal),
                FOREIGN KEY (source_note_id) REFERENCES note(id) ON DELETE CASCADE,
                FOREIGN KEY (target_note_id) REFERENCES note(id) ON DELETE SET NULL
            );
            CREATE INDEX IF NOT EXISTS idx_note_link_target ON note_link(target_note_id);

            CREATE TABLE IF NOT EXISTS note_link_index_state (
                note_id INTEGER NOT NULL PRIMARY KEY,
                indexed_updated_at INTEGER NOT NULL,
                FOREIGN KEY (note_id) REFERENCES note(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS workspace_link (
                id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                source_note_id INTEGER NULL,
                source_entry_id INTEGER NULL,
                target_note_id INTEGER NULL,
                target_board_id INTEGER NULL,
                target_card_id INTEGER NULL,
                target_entry_id INTEGER NULL,
                target_saved_view_id INTEGER NULL,
                origin TEXT NOT NULL,
                ordinal INTEGER NOT NULL,
                raw_target TEXT NULL,
                display_text TEXT NULL,
                start_byte INTEGER NULL,
                end_byte INTEGER NULL,
                line_number INTEGER NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (source_note_id) REFERENCES note(id) ON DELETE CASCADE,
                FOREIGN KEY (target_note_id) REFERENCES note(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_workspace_link_source_note ON workspace_link(source_note_id, origin, ordinal);
            CREATE INDEX IF NOT EXISTS idx_workspace_link_target_note ON workspace_link(target_note_id);

            CREATE TABLE IF NOT EXISTS workspace_link_index_state (
                source_kind TEXT NOT NULL,
                source_id INTEGER NOT NULL,
                indexed_content TEXT NOT NULL,
                PRIMARY KEY (source_kind, source_id)
            );

            CREATE TABLE IF NOT EXISTS workspace_reference_alias (
                id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                alias TEXT NOT NULL,
                normalized_alias TEXT NOT NULL,
                project_id INTEGER NULL,
                board_id INTEGER NULL,
                list_id INTEGER NULL,
                card_id INTEGER NULL,
                saved_view_id INTEGER NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (project_id) REFERENCES project(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_workspace_reference_alias_normalized ON workspace_reference_alias(normalized_alias);

            CREATE TABLE IF NOT EXISTS castle_change_revision (
                id INTEGER NOT NULL PRIMARY KEY,
                revision INTEGER NOT NULL DEFAULT 0,
                board_revision INTEGER NOT NULL DEFAULT 0,
                note_revision INTEGER NOT NULL DEFAULT 0,
                link_revision INTEGER NOT NULL DEFAULT 0
            );
            INSERT OR IGNORE INTO castle_change_revision (id, revision, board_revision, note_revision, link_revision)
            VALUES (1, 0, 0, 0, 0);

            CREATE VIRTUAL TABLE IF NOT EXISTS search_index USING fts5(
                item_type,
                item_id UNINDEXED,
                parent_id UNINDEXED,
                project_id UNINDEXED,
                title,
                body,
                tokenize = 'unicode61'
            );

            CREATE TRIGGER IF NOT EXISTS castle_track_note_insert
            AFTER INSERT ON note FOR EACH ROW BEGIN
                UPDATE castle_change_revision
                SET revision = revision + 1, note_revision = note_revision + 1
                WHERE id = 1;
            END;
            CREATE TRIGGER IF NOT EXISTS castle_track_note_update
            AFTER UPDATE ON note FOR EACH ROW BEGIN
                UPDATE castle_change_revision
                SET revision = revision + 1, note_revision = note_revision + 1
                WHERE id = 1;
            END;
            CREATE TRIGGER IF NOT EXISTS castle_track_note_delete
            AFTER DELETE ON note FOR EACH ROW BEGIN
                UPDATE castle_change_revision
                SET revision = revision + 1, note_revision = note_revision + 1
                WHERE id = 1;
            END;

            CREATE TRIGGER IF NOT EXISTS castle_track_project_insert
            AFTER INSERT ON project FOR EACH ROW BEGIN
                UPDATE castle_change_revision
                SET revision = revision + 1
                WHERE id = 1;
            END;
            CREATE TRIGGER IF NOT EXISTS castle_track_project_update
            AFTER UPDATE ON project FOR EACH ROW BEGIN
                UPDATE castle_change_revision
                SET revision = revision + 1
                WHERE id = 1;
            END;
            CREATE TRIGGER IF NOT EXISTS castle_track_project_delete
            AFTER DELETE ON project FOR EACH ROW BEGIN
                UPDATE castle_change_revision
                SET revision = revision + 1
                WHERE id = 1;
            END;

            CREATE TRIGGER IF NOT EXISTS castle_track_workspace_link_insert
            AFTER INSERT ON workspace_link FOR EACH ROW BEGIN
                UPDATE castle_change_revision
                SET revision = revision + 1, link_revision = link_revision + 1
                WHERE id = 1;
            END;
            CREATE TRIGGER IF NOT EXISTS castle_track_workspace_link_update
            AFTER UPDATE ON workspace_link FOR EACH ROW BEGIN
                UPDATE castle_change_revision
                SET revision = revision + 1, link_revision = link_revision + 1
                WHERE id = 1;
            END;
            CREATE TRIGGER IF NOT EXISTS castle_track_workspace_link_delete
            AFTER DELETE ON workspace_link FOR EACH ROW BEGIN
                UPDATE castle_change_revision
                SET revision = revision + 1, link_revision = link_revision + 1
                WHERE id = 1;
            END;

            CREATE TRIGGER IF NOT EXISTS castle_cleanup_note_workspace_link_index
            AFTER DELETE ON note FOR EACH ROW BEGIN
                DELETE FROM workspace_link_index_state
                WHERE source_kind = 'note' AND source_id = OLD.id;
            END;
            "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
            DROP TABLE IF EXISTS search_index;
            DROP TABLE IF EXISTS castle_change_revision;
            DROP TABLE IF EXISTS workspace_reference_alias;
            DROP TABLE IF EXISTS workspace_link_index_state;
            DROP TABLE IF EXISTS workspace_link;
            DROP TABLE IF EXISTS note_link_index_state;
            DROP TABLE IF EXISTS note_link;
            DROP TABLE IF EXISTS note_alias;
            DROP TABLE IF EXISTS note;
            DROP TABLE IF EXISTS project;
            "#,
        )
        .await?;
        Ok(())
    }
}
