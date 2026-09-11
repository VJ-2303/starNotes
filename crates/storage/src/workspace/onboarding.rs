use std::{
    fs::{self, OpenOptions},
    io::{ErrorKind, Write as _},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context as _, Result};
use entity::{note, note::Entity as Note, project::Entity as Project};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};

use crate::workspace::WorkspaceItem;

pub const DOCS_NOTE_TITLE: &str = "docs.md";

fn docs_content() -> String {
    r#"# Welcome to Castle

Castle is a local-first, lightweight Markdown note-taking app. This guide is an ordinary workspace note: edit it, move it, or delete it whenever you are ready.

## Quick Start

Press `Ctrl+P` for the command palette. It can create notes, open files, switch themes, search the workspace, and open settings. Type `new note: Brief` to create a note with a title.

Use `Ctrl+Shift+F` for full-text workspace search across all your notes. The **Home** screen gathers pinned notes and recently opened documents.

## Markdown & Editing

| Try this | What Castle does |
| --- | --- |
| Type `[[` and choose a note | Creates a navigable wikilink and tracks it in the **Links** inspector |
| Paste an image into a Markdown note | Copies it into local attachments and inserts portable Markdown |
| Add headings, tables, code, or Mermaid fences | Renders them in **Read** mode and builds a navigable outline |
| Open a Markdown, JSON, or text file | Edits the original file with matching syntax and outline support |

Use `Ctrl+Shift+O` for the outline and links inspector. In **Write** mode, `Alt+Shift+F` formats the current document; Markdown also supports smart list and task continuation, task toggling, line movement, and optional Vim editing. Use **Read**, **Side by side**, or **Write** as the default note view in **Settings → Editor → Markdown**.

## Links that stay useful

Type `[[` to complete references to notes. Castle displays readable labels in **Read** mode and remembers previous names when a note is renamed. Use the **Links** inspector to review outbound links and backlinks, and click a resolved link to navigate.

## Recover your work

Deleting a note or project moves it to **Trash** first. Use the undo action or open Trash to restore it; permanent deletion and **Empty Trash** are separate, explicit actions.

## Make it yours

Settings includes themes, typography, layout, editor behavior, shortcuts, and optional Vim mode. Castle is built to keep your thoughts fast, local, and organized.
"#.to_string()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FreshWorkspace {
    pub docs_note: WorkspaceItem,
    pub docs_path: PathBuf,
}

pub async fn seed_fresh_workspace(
    db: &(
         impl sea_orm::ConnectionTrait
         + sea_orm::TransactionTrait<Transaction = sea_orm::DatabaseTransaction>
     ),
    data_dir: &Path,
) -> Result<Option<FreshWorkspace>> {
    if workspace_has_items(db).await? {
        return Ok(None);
    }

    let transaction = db.begin().await?;

    let docs_content = docs_content();
    let docs_path = write_docs_file(data_dir, &docs_content)?;
    let now = now_ts();
    let note_result = note::ActiveModel {
        title: Set(DOCS_NOTE_TITLE.to_string()),
        project_id: Set(None),
        file_path: Set(Some(docs_path.to_string_lossy().into_owned())),
        file_managed_by_app: Set(true),
        cached_content: Set(docs_content.clone()),
        file_missing_since: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        last_opened_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&transaction)
    .await;

    let note = match note_result {
        Ok(note) => note,
        Err(err) => {
            remove_seed_file(&docs_path);
            return Err(err.into());
        }
    };

    if let Err(err) = transaction.commit().await {
        remove_seed_file(&docs_path);
        return Err(err.into());
    }
    crate::note::links::index_note_links(db, note.id, &docs_content, note.updated_at).await?;

    Ok(Some(FreshWorkspace {
        docs_note: WorkspaceItem {
            id: note.id as u32,
            title: note.title,
        },
        docs_path,
    }))
}

async fn workspace_has_items(
    db: &(
         impl sea_orm::ConnectionTrait
         + sea_orm::TransactionTrait<Transaction = sea_orm::DatabaseTransaction>
     ),
) -> Result<bool> {
    Ok(Project::find().one(db).await?.is_some() || Note::find().one(db).await?.is_some())
}

fn write_docs_file(data_dir: &Path, content: &str) -> Result<PathBuf> {
    let notes_dir = data_dir.join("notes");
    fs::create_dir_all(&notes_dir)
        .with_context(|| format!("failed to create {}", notes_dir.display()))?;

    for suffix in 1_u32.. {
        let file_name = if suffix == 1 {
            "docs.md".to_string()
        } else {
            format!("docs-{suffix}.md")
        };
        let path = notes_dir.join(file_name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                if let Err(err) = file.write_all(content.as_bytes()) {
                    drop(file);
                    remove_seed_file(&path);
                    return Err(err).with_context(|| format!("failed to write {}", path.display()));
                }
                return Ok(path);
            }
            Err(err) if err.kind() == ErrorKind::AlreadyExists => {}
            Err(err) => {
                return Err(err).with_context(|| format!("failed to create {}", path.display()));
            }
        }
    }

    unreachable!()
}

fn remove_seed_file(path: &Path) {
    if let Err(err) = fs::remove_file(path) {
        eprintln!(
            "Failed to clean up onboarding file {}: {err}",
            path.display()
        );
    }
}

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use migration::{Migrator, MigratorTrait};
    use sea_orm::Database;

    #[tokio::test]
    async fn seeds_a_docs_file_on_fresh_workspace() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let db = Database::connect("sqlite::memory:").await?;
        Migrator::up(&db, None).await?;

        let seeded = seed_fresh_workspace(&db, directory.path())
            .await?
            .context("fresh workspace should be seeded")?;

        assert_eq!(seeded.docs_note.title, DOCS_NOTE_TITLE);
        assert_eq!(seeded.docs_path, directory.path().join("notes/docs.md"));
        let seeded_docs = fs::read_to_string(&seeded.docs_path)?;
        assert!(seeded_docs.contains("# Welcome to Castle"));

        let stored_note = Note::find_by_id(i64::from(seeded.docs_note.id))
            .one(&db)
            .await?
            .context("seeded note should exist")?;
        assert!(stored_note.file_managed_by_app);
        assert_eq!(stored_note.cached_content, seeded_docs);
        assert!(stored_note.last_opened_at.is_some());

        Ok(())
    }

    #[tokio::test]
    async fn does_not_seed_a_workspace_that_already_has_content() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let db = Database::connect("sqlite::memory:").await?;
        Migrator::up(&db, None).await?;
        note::ActiveModel {
            title: Set("Existing".to_string()),
            project_id: Set(None),
            file_path: Set(None),
            file_managed_by_app: Set(false),
            cached_content: Set(String::new()),
            file_missing_since: Set(None),
            created_at: Set(1),
            updated_at: Set(1),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        assert!(seed_fresh_workspace(&db, directory.path()).await?.is_none());
        assert!(!directory.path().join("notes/docs.md").exists());
        assert_eq!(Note::find().all(&db).await?.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn preserves_an_existing_docs_file() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let notes_dir = directory.path().join("notes");
        fs::create_dir_all(&notes_dir)?;
        fs::write(notes_dir.join("docs.md"), "keep me")?;
        let db = Database::connect("sqlite::memory:").await?;
        Migrator::up(&db, None).await?;

        let seeded = seed_fresh_workspace(&db, directory.path())
            .await?
            .context("fresh workspace should be seeded")?;

        assert_eq!(fs::read_to_string(notes_dir.join("docs.md"))?, "keep me");
        assert_eq!(seeded.docs_path, notes_dir.join("docs-2.md"));
        Ok(())
    }
}
