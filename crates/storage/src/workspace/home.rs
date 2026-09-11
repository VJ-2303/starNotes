use anyhow::Result;
use sea_orm::{DbBackend, Statement};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceItemKind {
    Note,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceHomeItem {
    pub kind: WorkspaceItemKind,
    pub id: u32,
    pub title: String,
    pub project_id: Option<u32>,
    pub project_name: Option<String>,
    pub is_pinned: bool,
    pub last_opened_at: Option<i64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkspaceHomeState {
    pub pinned: Vec<WorkspaceHomeItem>,
    pub recent: Vec<WorkspaceHomeItem>,
}

pub async fn load_home(
    db: &(
         impl sea_orm::ConnectionTrait
         + sea_orm::TransactionTrait<Transaction = sea_orm::DatabaseTransaction>
     ),
) -> Result<WorkspaceHomeState> {
    let items = load_home_items(db).await?;
    let pinned = items
        .iter()
        .filter(|item| item.is_pinned)
        .cloned()
        .collect();
    let recent = items
        .into_iter()
        .filter(|item| !item.is_pinned && item.last_opened_at.is_some())
        .take(8)
        .collect();

    Ok(WorkspaceHomeState {
        pinned,
        recent,
    })
}

async fn load_home_items(
    db: &(
         impl sea_orm::ConnectionTrait
         + sea_orm::TransactionTrait<Transaction = sea_orm::DatabaseTransaction>
     ),
) -> Result<Vec<WorkspaceHomeItem>> {
    let rows = db
        .query_all_raw(Statement::from_string(
            DbBackend::Sqlite,
            r#"
            SELECT 'note' AS kind, n.id, n.title, n.project_id, p.name AS project_name,
                   n.is_pinned, n.last_opened_at
            FROM note n
            LEFT JOIN project p ON p.id = n.project_id
            WHERE n.deleted_at IS NULL AND (n.project_id IS NULL OR p.deleted_at IS NULL)
              AND (n.is_pinned = 1 OR n.last_opened_at IS NOT NULL)
            ORDER BY n.is_pinned DESC, COALESCE(n.last_opened_at, 0) DESC, n.title ASC
            "#,
        ))
        .await?;

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        items.push(WorkspaceHomeItem {
            kind: WorkspaceItemKind::Note,
            id: row.try_get::<i64>("", "id")? as u32,
            title: row.try_get("", "title")?,
            project_id: row
                .try_get::<Option<i64>>("", "project_id")?
                .map(|id| id as u32),
            project_name: row.try_get("", "project_name")?,
            is_pinned: row.try_get("", "is_pinned")?,
            last_opened_at: row.try_get("", "last_opened_at")?,
        });
    }
    Ok(items)
}

pub async fn mark_opened(
    db: &(
         impl sea_orm::ConnectionTrait
         + sea_orm::TransactionTrait<Transaction = sea_orm::DatabaseTransaction>
     ),
    kind: WorkspaceItemKind,
    id: u32,
    opened_at: i64,
) -> Result<()> {
    let table = match kind {
        WorkspaceItemKind::Note => "note",
    };
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        format!("UPDATE {table} SET last_opened_at = ? WHERE id = ? AND deleted_at IS NULL"),
        [opened_at.into(), (id as i64).into()],
    ))
    .await?;
    Ok(())
}

pub async fn set_pinned(
    db: &(
         impl sea_orm::ConnectionTrait
         + sea_orm::TransactionTrait<Transaction = sea_orm::DatabaseTransaction>
     ),
    kind: WorkspaceItemKind,
    id: u32,
    pinned: bool,
) -> Result<()> {
    let table = match kind {
        WorkspaceItemKind::Note => "note",
    };
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        format!("UPDATE {table} SET is_pinned = ? WHERE id = ? AND deleted_at IS NULL"),
        [pinned.into(), (id as i64).into()],
    ))
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use entity::{note, project};
    use migration::{Migrator, MigratorTrait};
    use sea_orm::{ActiveModelTrait, ActiveValue::Set, Database};

    #[tokio::test]
    async fn home_separates_pinned_from_recent_notes() -> Result<()> {
        let db = Database::connect("sqlite::memory:").await?;
        Migrator::up(&db, None).await?;

        let project = project::ActiveModel {
            name: Set("Castle".to_string()),
            archived: Set(false),
            position: Set(0),
            ..Default::default()
        }
        .insert(&db)
        .await?;
        let pinned_note = note::ActiveModel {
            title: Set("Pinned note".to_string()),
            project_id: Set(Some(project.id)),
            file_path: Set(None),
            file_managed_by_app: Set(false),
            cached_content: Set(String::new()),
            file_missing_since: Set(None),
            created_at: Set(1),
            updated_at: Set(1),
            is_pinned: Set(true),
            last_opened_at: Set(Some(5)),
            ..Default::default()
        }
        .insert(&db)
        .await?;
        let recent_note = note::ActiveModel {
            title: Set("Recent note".to_string()),
            project_id: Set(Some(project.id)),
            file_path: Set(None),
            file_managed_by_app: Set(false),
            cached_content: Set(String::new()),
            file_missing_since: Set(None),
            created_at: Set(2),
            updated_at: Set(2),
            is_pinned: Set(false),
            last_opened_at: Set(Some(10)),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        let home = load_home(&db).await?;
        assert_eq!(home.pinned.len(), 1);
        assert_eq!(home.pinned[0].id, pinned_note.id as u32);
        assert_eq!(home.recent.len(), 1);
        assert_eq!(home.recent[0].id, recent_note.id as u32);
        Ok(())
    }
}
