use std::{future::Future, pin::Pin, sync::Arc};

use crate::workspace::api::{
    CreateNoteInput, CreateProjectInput, MoveNoteInput, NoteDetail, ProjectSummary,
    RenameProjectInput, UpdateNoteInput,
};
use anyhow::Result;
use sea_orm::{
    ConnectionTrait, DatabaseTransaction, DbBackend, Statement, TransactionTrait,
};

use crate::store::Store;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MutationOrigin {
    LocalApp,
    ExternalAgent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChangeDomain {
    Workspace,
    Note,
    Link,
}

async fn record_change_in_connection(
    db: &impl ConnectionTrait,
    domain: ChangeDomain,
) -> Result<()> {
    let assignments = match domain {
        ChangeDomain::Workspace => "revision = revision + 1",
        ChangeDomain::Note => "revision = revision + 1, note_revision = note_revision + 1",
        ChangeDomain::Link => {
            "revision = revision + 1, note_revision = note_revision + 1, link_revision = link_revision + 1"
        }
    };
    db.execute_raw(Statement::from_string(
        DbBackend::Sqlite,
        format!("UPDATE castle_change_revision SET {assignments} WHERE id = 1"),
    ))
    .await?;
    Ok(())
}

impl Store {
    pub fn mutations(&self, origin: MutationOrigin) -> Mutations {
        Mutations {
            store: self.clone(),
            origin,
        }
    }
}

#[derive(Clone)]
pub struct Mutations {
    store: Store,
    origin: MutationOrigin,
}

type MutationFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

impl Mutations {
    async fn execute<T, F>(&self, domain: ChangeDomain, operation: F) -> Result<T>
    where
        T: Send,
        F: for<'a> FnOnce(&'a Store<DatabaseTransaction>) -> MutationFuture<'a, T>,
    {
        let transaction = Arc::new(self.store.db.as_ref().begin().await?);
        let transactional_store = Store {
            db: transaction.clone(),
        };
        let result = operation(&transactional_store).await?;
        if self.origin == MutationOrigin::ExternalAgent {
            record_change_in_connection(transactional_store.db.as_ref(), domain).await?;
        }
        drop(transactional_store);
        let transaction = Arc::try_unwrap(transaction)
            .map_err(|_| anyhow::anyhow!("storage transaction remained shared after mutation"))?;
        transaction.commit().await?;
        Ok(result)
    }

    pub async fn create_note(&self, input: CreateNoteInput) -> Result<NoteDetail> {
        self.execute(ChangeDomain::Link, move |store| {
            Box::pin(store.create_note(input))
        })
        .await
    }

    pub async fn update_note(&self, input: UpdateNoteInput) -> Result<NoteDetail> {
        self.execute(ChangeDomain::Link, move |store| {
            Box::pin(store.update_note(input))
        })
        .await
    }

    pub async fn move_note(&self, input: MoveNoteInput) -> Result<NoteDetail> {
        self.execute(ChangeDomain::Note, move |store| {
            Box::pin(store.move_note(input))
        })
        .await
    }

    pub async fn create_project(&self, input: CreateProjectInput) -> Result<ProjectSummary> {
        self.execute(ChangeDomain::Workspace, move |store| {
            Box::pin(store.create_project(input))
        })
        .await
    }

    pub async fn rename_project(&self, input: RenameProjectInput) -> Result<ProjectSummary> {
        self.execute(ChangeDomain::Workspace, move |store| {
            Box::pin(store.rename_project(input))
        })
        .await
    }
}
