use super::*;
use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DbBackend, PaginatorTrait, Statement};

use crate::MutationOrigin;

async fn store() -> Result<Store> {
    let db = Database::connect("sqlite::memory:").await?;
    Migrator::up(&db, None).await?;
    Ok(Store::new(db))
}

#[tokio::test]
async fn creates_and_moves_notes_in_projects() -> Result<()> {
    let store = store().await?;
    let project = store
        .create_project(CreateProjectInput {
            name: "Work Notes".to_string(),
        })
        .await?;
    let note = store
        .create_note(CreateNoteInput {
            title: "Meeting Notes".to_string(),
            content: "Discuss roadmap".to_string(),
            project_id: Some(project.id),
        })
        .await?;
    assert_eq!(note.title, "Meeting Notes");
    assert_eq!(note.project_id, Some(project.id));

    let moved = store
        .move_note(MoveNoteInput {
            note_id: note.id,
            project_id: None,
        })
        .await?;
    assert_eq!(moved.project_id, None);
    Ok(())
}

#[tokio::test]
async fn local_mutations_do_not_bump_and_external_mutations_bump_the_owned_domain() -> Result<()> {
    let store = store().await?;
    let project = store
        .mutations(MutationOrigin::LocalApp)
        .create_project(CreateProjectInput {
            name: "Revision".to_string(),
        })
        .await?;
    let note = store
        .mutations(MutationOrigin::LocalApp)
        .create_note(CreateNoteInput {
            title: "Watcher regression".to_string(),
            content: String::new(),
            project_id: Some(project.id),
        })
        .await?;
    store
        .db
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "UPDATE note SET last_opened_at = ? WHERE id = ?",
            [123_i64.into(), note.id.into()],
        ))
        .await?;

    let row = change_revision_row(&store).await?;
    assert_eq!(row.try_get::<i64>("", "revision")?, 0);
    assert_eq!(row.try_get::<i64>("", "note_revision")?, 0);

    store
        .mutations(MutationOrigin::ExternalAgent)
        .move_note(MoveNoteInput {
            note_id: note.id,
            project_id: None,
        })
        .await?;
    let row = change_revision_row(&store).await?;
    assert_eq!(row.try_get::<i64>("", "revision")?, 1);
    assert_eq!(row.try_get::<i64>("", "note_revision")?, 1);
    Ok(())
}

#[tokio::test]
async fn failed_revision_bump_rolls_back_the_data_mutation() -> Result<()> {
    let store = store().await?;
    store
        .db
        .execute_raw(Statement::from_string(
            DbBackend::Sqlite,
            "CREATE TRIGGER fail_revision_bump BEFORE UPDATE ON castle_change_revision BEGIN SELECT RAISE(ABORT, 'forced revision failure'); END",
        ))
        .await?;

    let result = store
        .mutations(MutationOrigin::ExternalAgent)
        .create_project(CreateProjectInput {
            name: "Must roll back".to_string(),
        })
        .await;

    assert!(result.is_err());
    assert_eq!(Project::find().count(store.db.as_ref()).await?, 0);
    let row = change_revision_row(&store).await?;
    assert_eq!(row.try_get::<i64>("", "revision")?, 0);
    Ok(())
}

async fn change_revision_row(store: &Store) -> Result<sea_orm::QueryResult> {
    let row = store
        .db
        .query_one_raw(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT revision, board_revision, note_revision, link_revision FROM castle_change_revision WHERE id = 1",
        ))
        .await?
        .context("revision row was not found")?;
    Ok(row)
}

#[tokio::test]
async fn search_notes_returns_only_matching_hits() -> Result<()> {
    let store = store().await?;
    let project = store
        .create_project(CreateProjectInput {
            name: "Search".to_string(),
        })
        .await?;
    let matching = store
        .create_note(CreateNoteInput {
            title: "Unique needle note".to_string(),
            content: "Some interesting content".to_string(),
            project_id: Some(project.id),
        })
        .await?;
    store
        .create_note(CreateNoteInput {
            title: "Unrelated note".to_string(),
            content: "Nothing here".to_string(),
            project_id: Some(project.id),
        })
        .await?;

    let hits = store
        .search_notes(SearchNotesInput {
            query: "needle".to_string(),
            project_id: None,
            limit: None,
        })
        .await?;
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, matching.id);
    assert_eq!(hits[0].title, "Unique needle note");
    Ok(())
}
