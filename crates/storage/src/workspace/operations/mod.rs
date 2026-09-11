use std::{
    collections::HashMap,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};
use entity::{note, note::Entity as Note, project, project::Entity as Project};
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::Set,
    ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, TransactionSession, TransactionTrait,
};

use crate::workspace::api::{
    CreateNoteInput, CreateProjectInput, MoveNoteInput, NoteDetail, NoteLinkDetail,
    NoteLinksDetail, NoteSummary, ProjectSummary, RelatedItemDetail, RenameProjectInput,
    SearchNotesInput, UpdateNoteInput,
};

use crate::store::Store;

mod mapping;
mod notes;
mod projects;
mod relations;
mod validation;

use mapping::*;
use validation::required_text;

impl<C> Store<C>
where
    C: ConnectionTrait + TransactionTrait + Send + Sync + 'static,
{
    async fn active_note(&self, note_id: i64) -> Result<note::Model> {
        let note = Note::find_by_id(note_id)
            .filter(note::Column::DeletedAt.is_null())
            .one(self.db.as_ref())
            .await?
            .with_context(|| format!("active note {note_id} was not found"))?;
        if let Some(project_id) = note.project_id {
            self.active_project(project_id).await?;
        }
        Ok(note)
    }

    async fn note_detail(&self, note: note::Model) -> Result<NoteDetail> {
        let project_name = match note.project_id {
            Some(project_id) => Some(self.active_project(project_id).await?.name),
            None => None,
        };
        let (content, file_missing) = match note.file_path.as_ref() {
            Some(path) => match tokio::fs::read_to_string(path).await {
                Ok(content) => (content, false),
                Err(_) => (note.cached_content.clone(), true),
            },
            None => (note.cached_content.clone(), false),
        };
        let related_items = self.related_items_for_note(note.id).await?;
        Ok(NoteDetail {
            id: note.id,
            title: note.title,
            content,
            project_id: note.project_id,
            project_name,
            file_path: note.file_path,
            file_managed_by_app: note.file_managed_by_app,
            file_missing: file_missing || note.file_missing_since.is_some(),
            is_pinned: note.is_pinned,
            created_at: note.created_at,
            updated_at: note.updated_at,
            related_items,
        })
    }

    async fn active_project(&self, project_id: i64) -> Result<project::Model> {
        Project::find_by_id(project_id)
            .filter(project::Column::Archived.eq(false))
            .filter(project::Column::DeletedAt.is_null())
            .one(self.db.as_ref())
            .await?
            .with_context(|| format!("active project {project_id} was not found"))
    }

    async fn active_project_map(&self) -> Result<HashMap<i64, String>> {
        Ok(Project::find()
            .filter(project::Column::Archived.eq(false))
            .filter(project::Column::DeletedAt.is_null())
            .all(self.db.as_ref())
            .await?
            .into_iter()
            .map(|project| (project.id, project.name))
            .collect())
    }
}

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

fn next_updated_at(current: i64) -> i64 {
    std::cmp::max(now_ts(), current.saturating_add(1))
}

#[cfg(test)]
mod tests;
