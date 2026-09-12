use super::*;
use entity::{note, project};
use migration::{Migrator, MigratorTrait};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, Database, EntityTrait, PaginatorTrait,
};

#[tokio::test]
async fn rename_operations_record_historical_reference_aliases_transactionally() -> Result<()> {
    let db = Database::connect("sqlite::memory:").await?;
    Migrator::up(&db, None).await?;
    let project = project::ActiveModel {
        name: Set("Old Project".to_string()),
        archived: Set(false),
        position: Set(0),
        ..Default::default()
    }
    .insert(&db)
    .await?;
    let note = note::ActiveModel {
        title: Set("Old Note".to_string()),
        project_id: Set(Some(project.id)),
        file_path: Set(None),
        file_managed_by_app: Set(false),
        cached_content: Set(String::new()),
        file_missing_since: Set(None),
        created_at: Set(0),
        updated_at: Set(0),
        ..Default::default()
    }
    .insert(&db)
    .await?;

    crate::workspace::rename_project(&db, project.id as u32, "New Project".to_string()).await?;
    crate::workspace::persist_workspace_title(
        &db,
        crate::workspace::WorkspaceTitleTarget::Note(note.id as u32),
        "New Note".to_string(),
    )
    .await?;

    let catalog = load_workspace_reference_catalog(&db).await?;
    assert_eq!(
        resolve_reference_target("note:Old Project / New Note", &catalog),
        Ok(ResolvedWorkspaceReference::Item(WorkspaceItemRef {
            kind: WorkspaceItemKind::Note,
            id: note.id,
        }))
    );
    Ok(())
}

#[tokio::test]
async fn linked_note_creation_rolls_back_when_the_relationship_is_invalid() -> Result<()> {
    let db = Database::connect("sqlite::memory:").await?;
    Migrator::up(&db, None).await?;

    let result = crate::workspace::create_managed_linked_note(
        &db,
        None,
        "Draft".to_string(),
        "draft.md".to_string(),
        "# Draft".to_string(),
        WorkspaceItemRef {
            kind: WorkspaceItemKind::Note,
            id: 999,
        },
    )
    .await;

    assert!(result.is_err());
    assert_eq!(Note::find().count(&db).await?, 0);
    assert_eq!(WorkspaceLink::find().count(&db).await?, 0);
    assert_eq!(WorkspaceLinkIndexState::find().count(&db).await?, 0);
    Ok(())
}

#[tokio::test]
async fn repair_batches_never_index_more_than_the_requested_bound() -> Result<()> {
    let db = Database::connect("sqlite::memory:").await?;
    Migrator::up(&db, None).await?;
    for id in 1..=5 {
        note::ActiveModel {
            id: Set(id),
            title: Set(format!("Note {id}")),
            cached_content: Set("[[Missing]]".to_string()),
            file_managed_by_app: Set(false),
            created_at: Set(id),
            updated_at: Set(id),
            ..Default::default()
        }
        .insert(&db)
        .await?;
    }

    let first = repair_workspace_link_index_batch(&db, 2).await?;
    assert_eq!(
        first.indexed_notes + first.indexed_workspace_notes + first.indexed_entries,
        2
    );
    assert!(first.has_more);

    let second = repair_workspace_link_index_batch(&db, 2).await?;
    assert_eq!(
        second.indexed_notes + second.indexed_workspace_notes + second.indexed_entries,
        2
    );
    let third = repair_workspace_link_index_batch(&db, 2).await?;
    assert_eq!(
        third.indexed_notes + third.indexed_workspace_notes + third.indexed_entries,
        1
    );
    assert!(!third.has_more);
    Ok(())
}
