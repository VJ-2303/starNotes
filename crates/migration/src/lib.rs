pub use sea_orm_migration::prelude::*;

mod m20260101_000001_initial_schema;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260101_000001_initial_schema::Migration),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ConnectionTrait, Database, DbBackend, Statement};

    #[tokio::test]
    async fn initial_schema_migration_creates_required_tables() -> Result<(), DbErr> {
        let db = Database::connect("sqlite::memory:").await?;
        Migrator::up(&db, None).await?;

        for table in [
            "project",
            "note",
            "note_alias",
            "note_link",
            "note_link_index_state",
            "workspace_link",
            "workspace_link_index_state",
            "workspace_reference_alias",
            "castle_change_revision",
            "search_index",
        ] {
            let row = db
                .query_one_raw(Statement::from_string(
                    DbBackend::Sqlite,
                    format!("SELECT COUNT(*) AS count FROM {table}"),
                ))
                .await?
                .ok_or_else(|| DbErr::Custom(format!("failed to query {table}")))?;
            let _count = row.try_get::<i64>("", "count")?;
        }

        let revision_row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT revision FROM castle_change_revision WHERE id = 1",
            ))
            .await?
            .ok_or_else(|| DbErr::Custom("missing revision row".to_string()))?;
        assert_eq!(revision_row.try_get::<i64>("", "revision")?, 0);

        Ok(())
    }
}
