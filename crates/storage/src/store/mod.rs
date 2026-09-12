use std::{future::Future, pin::Pin, sync::Arc};

use anyhow::Result;
use sea_orm::{
    AccessMode, ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend, DbErr,
    ExecResult, IsolationLevel, QueryResult, Statement, TransactionError, TransactionOptions,
    TransactionTrait,
};

use migration::{Migrator, MigratorTrait};

mod mutations;

pub use mutations::{MutationOrigin, Mutations};

#[derive(Clone)]
pub struct Store<C = DatabaseConnection> {
    pub(crate) db: Arc<C>,
}

impl From<&Store> for Store {
    fn from(store: &Store) -> Self {
        store.clone()
    }
}

#[cfg(any(test, feature = "test-support"))]
impl From<DatabaseConnection> for Store {
    fn from(db: DatabaseConnection) -> Self {
        Self { db: Arc::new(db) }
    }
}

#[cfg(any(test, feature = "test-support"))]
impl From<&DatabaseConnection> for Store {
    fn from(db: &DatabaseConnection) -> Self {
        Self {
            db: Arc::new(db.clone()),
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
impl From<Arc<DatabaseConnection>> for Store {
    fn from(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[derive(Clone, Debug)]
pub struct StoreOptions {
    database_url: String,
    min_connections: u32,
    max_connections: u32,
}

impl StoreOptions {
    pub fn new(database_url: impl Into<String>) -> Self {
        Self {
            database_url: database_url.into(),
            min_connections: 1,
            max_connections: 4,
        }
    }

    pub fn connection_pool(mut self, min_connections: u32, max_connections: u32) -> Self {
        self.min_connections = min_connections;
        self.max_connections = std::cmp::max(max_connections, min_connections);
        self
    }
}

impl Store {
    pub async fn connect(options: StoreOptions) -> Result<Self> {
        let mut connect_options = ConnectOptions::new(options.database_url);
        connect_options
            .min_connections(options.min_connections)
            .max_connections(options.max_connections);
        let db = Database::connect(connect_options).await?;
        ensure_migrated(&db).await?;
        Ok(Self { db: Arc::new(db) })
    }

    #[cfg(test)]
    pub(crate) fn new(db: DatabaseConnection) -> Self {
        Self { db: Arc::new(db) }
    }
}

#[async_trait::async_trait]
impl<C> ConnectionTrait for Store<C>
where
    C: ConnectionTrait + Send + Sync,
{
    fn get_database_backend(&self) -> DbBackend {
        self.db.get_database_backend()
    }

    async fn execute_raw(&self, statement: Statement) -> Result<ExecResult, DbErr> {
        self.db.execute_raw(statement).await
    }

    async fn execute_unprepared(&self, sql: &str) -> Result<ExecResult, DbErr> {
        self.db.execute_unprepared(sql).await
    }

    async fn query_one_raw(&self, statement: Statement) -> Result<Option<QueryResult>, DbErr> {
        self.db.query_one_raw(statement).await
    }

    async fn query_all_raw(&self, statement: Statement) -> Result<Vec<QueryResult>, DbErr> {
        self.db.query_all_raw(statement).await
    }

    fn support_returning(&self) -> bool {
        self.db.support_returning()
    }

    fn is_mock_connection(&self) -> bool {
        self.db.is_mock_connection()
    }
}

#[async_trait::async_trait]
impl<C> TransactionTrait for Store<C>
where
    C: TransactionTrait + Send + Sync,
{
    type Transaction = C::Transaction;

    async fn begin(&self) -> Result<Self::Transaction, DbErr> {
        self.db.begin().await
    }

    async fn begin_with_config(
        &self,
        isolation_level: Option<IsolationLevel>,
        access_mode: Option<AccessMode>,
    ) -> Result<Self::Transaction, DbErr> {
        self.db
            .begin_with_config(isolation_level, access_mode)
            .await
    }

    async fn begin_with_options(
        &self,
        options: TransactionOptions,
    ) -> Result<Self::Transaction, DbErr> {
        self.db.begin_with_options(options).await
    }

    async fn transaction<F, T, E>(&self, callback: F) -> Result<T, TransactionError<E>>
    where
        F: for<'a> FnOnce(
                &'a Self::Transaction,
            ) -> Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'a>>
            + Send,
        T: Send,
        E: std::fmt::Display + std::fmt::Debug + Send,
    {
        self.db.transaction(callback).await
    }

    async fn transaction_with_config<F, T, E>(
        &self,
        callback: F,
        isolation_level: Option<IsolationLevel>,
        access_mode: Option<AccessMode>,
    ) -> Result<T, TransactionError<E>>
    where
        F: for<'a> FnOnce(
                &'a Self::Transaction,
            ) -> Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'a>>
            + Send,
        T: Send,
        E: std::fmt::Display + std::fmt::Debug + Send,
    {
        self.db
            .transaction_with_config(callback, isolation_level, access_mode)
            .await
    }
}

async fn ensure_migrated(db: &DatabaseConnection) -> Result<()> {
    if should_reset_legacy_schema(db).await? {
        reset_sqlite_database(db).await?;
    }
    match Migrator::up(db, None).await {
        Ok(_) => Ok(()),
        Err(err) => {
            let msg = err.to_string();
            if msg.contains("is missing, this migration has been applied")
                || msg.contains("no such table")
            {
                reset_sqlite_database(db).await?;
                Migrator::up(db, None).await?;
                Ok(())
            } else {
                Err(err.into())
            }
        }
    }
}

async fn should_reset_legacy_schema(db: &DatabaseConnection) -> Result<bool> {
    let legacy_table = db
        .query_one_raw(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name IN ('board', 'card', 'entry', 'saved_board_view', 'board_template', 'workflow', 'recurring_task')",
        ))
        .await?;
    if legacy_table.is_some() {
        return Ok(true);
    }

    let row = db
        .query_one_raw(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='seaql_migrations'",
        ))
        .await?;
    if row.is_none() {
        return Ok(false);
    }
    let legacy_rows = db
        .query_all_raw(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT version FROM seaql_migrations WHERE version != 'm20260101_000001_initial_schema'",
        ))
        .await?;
    Ok(!legacy_rows.is_empty())
}

async fn reset_sqlite_database(db: &DatabaseConnection) -> Result<()> {
    db.execute_unprepared("PRAGMA foreign_keys = OFF;").await?;

    let triggers = db
        .query_all_raw(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT name FROM sqlite_master WHERE type='trigger' AND name NOT LIKE 'sqlite_%'",
        ))
        .await?;
    for row in triggers {
        let name: String = row.try_get("", "name")?;
        db.execute_unprepared(&format!("DROP TRIGGER IF EXISTS \"{name}\";")).await?;
    }

    let views = db
        .query_all_raw(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT name FROM sqlite_master WHERE type='view' AND name NOT LIKE 'sqlite_%'",
        ))
        .await?;
    for row in views {
        let name: String = row.try_get("", "name")?;
        db.execute_unprepared(&format!("DROP VIEW IF EXISTS \"{name}\";")).await?;
    }

    let tables = db
        .query_all_raw(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
        ))
        .await?;
    for row in tables {
        let name: String = row.try_get("", "name")?;
        db.execute_unprepared(&format!("DROP TABLE IF EXISTS \"{name}\";")).await?;
    }

    db.execute_unprepared("PRAGMA foreign_keys = ON;").await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ensure_migrated_fresh_database() -> Result<()> {
        let db = Database::connect("sqlite::memory:").await?;
        ensure_migrated(&db).await?;
        assert!(!should_reset_legacy_schema(&db).await?);
        Ok(())
    }

    #[tokio::test]
    async fn ensure_migrated_resets_legacy_database() -> Result<()> {
        let db = Database::connect("sqlite::memory:").await?;
        db.execute_unprepared(
            "CREATE TABLE seaql_migrations (version TEXT PRIMARY KEY, applied_at INTEGER NOT NULL);",
        )
        .await?;
        db.execute_unprepared(
            "INSERT INTO seaql_migrations (version, applied_at) VALUES ('m20220101_000001_create_table', 1);",
        )
        .await?;
        db.execute_unprepared("CREATE TABLE legacy_trash (id INTEGER PRIMARY KEY);")
            .await?;

        ensure_migrated(&db).await?;

        let legacy_table = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='legacy_trash'",
            ))
            .await?;
        assert!(legacy_table.is_none());

        let note_table = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='note'",
            ))
            .await?;
        assert!(note_table.is_some());

        assert!(!should_reset_legacy_schema(&db).await?);
        Ok(())
    }
}
