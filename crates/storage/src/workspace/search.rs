use entity::{note, note::Entity as Note, project, project::Entity as Project};
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, DbBackend, DbErr, EntityTrait, QueryFilter,
    QuerySelect, Statement, Value,
    sea_query::{Query, SelectStatement},
};

const SEARCH_INSERT_BODY_BUDGET: usize = 1024 * 1024;
const SEARCH_INSERT_DOCUMENT_LIMIT: usize = 100;
const SEARCH_PREVIEW_CHARS: u32 = 5000;
const SEARCH_PREVIEW_LOOKBACK: u32 = 1200;
const SEARCH_PREVIEW_WINDOW: u32 = SEARCH_PREVIEW_CHARS + SEARCH_PREVIEW_LOOKBACK;
const SEARCH_ANCHOR_BODY_LIMIT: u32 = 65536;
const SEARCH_ANCHOR_DEPTH_LIMIT: u32 = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchResultKind {
    Note,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchResult {
    pub kind: SearchResultKind,
    pub item_id: u32,
    pub open_id: u32,
    pub project_id: Option<u32>,
    pub title: String,
    pub parent_title: Option<String>,
    pub highlighted_title: String,
    pub snippet: String,
    pub preview: String,
}

fn active_project_ids_query() -> SelectStatement {
    Query::select()
        .column(project::Column::Id)
        .from(Project)
        .and_where(project::Column::DeletedAt.is_null())
        .to_owned()
}

pub async fn rebuild_search_index(
    db: &(
         impl sea_orm::ConnectionTrait
         + sea_orm::TransactionTrait<Transaction = sea_orm::DatabaseTransaction>
     ),
) -> Result<(), DbErr> {
    let notes = Note::find()
        .filter(note::Column::DeletedAt.is_null())
        .filter(
            Condition::any()
                .add(note::Column::ProjectId.is_null())
                .add(note::Column::ProjectId.in_subquery(active_project_ids_query())),
        )
        .select_only()
        .column(note::Column::Id)
        .column(note::Column::ProjectId)
        .column(note::Column::Title)
        .column(note::Column::CachedContent)
        .into_tuple::<(i64, Option<i64>, String, String)>()
        .all(db)
        .await?;

    let txn = db.begin().await?;
    txn.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "DELETE FROM search_index",
        [],
    ))
    .await?;

    insert_search_documents(
        &txn,
        notes
            .into_iter()
            .map(|(id, project_id, title, content)| SearchDocument {
                item_type: "note",
                item_id: id,
                parent_id: Some(id),
                project_id,
                title,
                body: content,
            }),
    )
    .await?;

    txn.commit().await?;

    Ok(())
}

pub async fn search_workspace(
    db: &(
         impl sea_orm::ConnectionTrait
         + sea_orm::TransactionTrait<Transaction = sea_orm::DatabaseTransaction>
     ),
    query: &str,
    limit: u32,
) -> Result<Vec<SearchResult>, DbErr> {
    let rows = if let Some(match_query) = fts_query(query) {
        db.query_all_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            search_match_sql(&preview_anchor_terms(&match_query)),
            [match_query.into(), (limit as i64).into()],
        ))
        .await?
    } else {
        db.query_all_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT
                item_type,
                item_id,
                COALESCE(parent_id, item_id) AS open_id,
                project_id,
                title,
                title AS highlighted_title,
                CASE
                    WHEN body = '' THEN title
                    ELSE substr(body, 1, 160)
                END AS snippet,
                substr(body, 1, 8000) AS preview
             FROM search_index
             WHERE item_type = 'note'
             ORDER BY rowid DESC
             LIMIT ?",
            [(limit as i64).into()],
        ))
        .await?
    };

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let mut results = Vec::with_capacity(rows.len());
    for row in rows {
        let item_type: String = row.try_get("", "item_type")?;
        if item_type != "note" {
            continue;
        }

        let item_id: i64 = row.try_get("", "item_id")?;
        let open_id: i64 = row.try_get("", "open_id")?;
        let project_id: Option<i64> = row.try_get("", "project_id")?;

        results.push(SearchResult {
            kind: SearchResultKind::Note,
            item_id: item_id as u32,
            open_id: open_id as u32,
            project_id: project_id.map(|id| id as u32),
            parent_title: None,
            title: row.try_get("", "title")?,
            highlighted_title: row.try_get("", "highlighted_title")?,
            snippet: row.try_get("", "snippet")?,
            preview: row.try_get("", "preview")?,
        });
    }

    Ok(results)
}

fn preview_anchor_terms(match_query: &str) -> Vec<String> {
    match_query
        .split(' ')
        .filter_map(|term| {
            let term = term
                .strip_suffix('*')
                .unwrap_or(term)
                .to_ascii_lowercase()
                .replace('\'', "''");
            (!term.is_empty()).then_some(term)
        })
        .collect()
}

fn window_term_score(body: &str, candidate: &str, terms: &[String]) -> String {
    let mut hits = Vec::with_capacity(terms.len());
    for term in terms.iter() {
        hits.push(format!(
            "(instr(substr({body}, max(({candidate}) - {lookback}, 1), {window}), char(1) || '{term}') > 0)",
            lookback = SEARCH_PREVIEW_LOOKBACK,
            window = SEARCH_PREVIEW_WINDOW,
        ));
    }
    format!(
        "CASE WHEN ({candidate}) > 0 THEN ({hits}) ELSE -1 END",
        hits = hits.join(" + "),
    )
}

fn first_occurrence_anchor(terms: &[String], candidates: &[String]) -> String {
    let mut scores = Vec::with_capacity(candidates.len());
    for candidate in candidates.iter() {
        scores.push(window_term_score("highlighted_body", candidate, terms));
    }
    let mut keys = Vec::with_capacity(candidates.len());
    for (candidate, score) in candidates.iter().zip(scores.iter()) {
        keys.push(format!("(({score}) * 1000000000 + ({candidate}))"));
    }
    let max_key = keys.join(", ");
    let mut branches = String::new();
    for (candidate, key) in candidates.iter().zip(keys.iter()) {
        branches.push_str(&format!(
            "WHEN ({candidate}) > 0 AND ({key}) = max({max_key}) THEN max(({candidate}) - {lookback}, 1) ",
            lookback = SEARCH_PREVIEW_LOOKBACK,
        ));
    }
    format!("CASE {branches}ELSE NULL END")
}

fn best_occurrence_anchor(terms: &[String], longest: &str) -> String {
    let score = window_term_score("matched.highlighted_body", "occ.pos", terms);
    format!(
        "CASE WHEN length(matched.highlighted_body) <= {body_limit} THEN (
            WITH RECURSIVE occ(pos, depth) AS (
                SELECT instr(matched.highlighted_body, char(1) || '{longest}'), 1
                UNION ALL
                SELECT occ.pos + instr(substr(matched.highlighted_body, occ.pos + 1), char(1) || '{longest}'), occ.depth + 1
                FROM occ
                WHERE occ.pos > 0
                  AND occ.depth < {depth_limit}
                  AND instr(substr(matched.highlighted_body, occ.pos + 1), char(1) || '{longest}') > 0
            )
            SELECT max(scored.pos - {lookback}, 1) FROM (
                SELECT occ.pos AS pos, ({score}) AS s FROM occ WHERE occ.pos > 0
            ) AS scored
            ORDER BY scored.s DESC, scored.pos DESC
            LIMIT 1
        ) ELSE NULL END",
        body_limit = SEARCH_ANCHOR_BODY_LIMIT,
        depth_limit = SEARCH_ANCHOR_DEPTH_LIMIT,
        lookback = SEARCH_PREVIEW_LOOKBACK,
    )
}

fn search_match_sql(terms: &[String]) -> String {
    let mut candidates = Vec::with_capacity(terms.len());
    for term in terms {
        candidates.push(format!("instr(highlighted_body, char(1) || '{term}')"));
    }
    let longest = terms.iter().max_by_key(|term| term.chars().count());
    let anchor = match (candidates.len(), longest) {
        (1, _) => format!(
            "CASE WHEN ({candidate}) > 0 THEN max(({candidate}) - {lookback}, 1) ELSE 1 END",
            candidate = candidates[0],
            lookback = SEARCH_PREVIEW_LOOKBACK,
        ),
        (_, Some(longest)) => {
            let enumerated = best_occurrence_anchor(terms, longest);
            let fallback = first_occurrence_anchor(terms, &candidates);
            format!("COALESCE(({enumerated}), ({fallback}), 1)")
        }
        _ => "1".to_string(),
    };
    format!(
        "SELECT
            item_type,
            item_id,
            open_id,
            project_id,
            title,
            highlighted_title,
            snippet,
            substr(highlighted_body, {anchor}, {preview}) AS preview
         FROM (
            SELECT
                item_type,
                item_id,
                COALESCE(parent_id, item_id) AS open_id,
                project_id,
                title,
                highlight(search_index, 4, char(1), char(2)) AS highlighted_title,
                snippet(search_index, 5, char(1), char(2), '...', 18) AS snippet,
                lower(highlight(search_index, 5, char(1), char(2))) AS highlighted_body
            FROM search_index
            WHERE search_index MATCH ?
              AND item_type = 'note'
            ORDER BY bm25(search_index)
            LIMIT ?
        ) AS matched",
        preview = SEARCH_PREVIEW_CHARS,
    )
}

fn fts_query(query: &str) -> Option<String> {
    let raw_terms = query
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
        .filter(|term| !term.is_empty())
        .collect::<Vec<_>>();

    let multi_term = raw_terms.len() > 1;
    let mut terms = raw_terms
        .iter()
        .filter_map(|term| fts_query_term(term, multi_term))
        .collect::<Vec<_>>();

    if terms.is_empty() {
        terms = raw_terms.iter().map(|term| (*term).to_string()).collect();
    }

    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" "))
    }
}

fn fts_query_term(term: &str, multi_term: bool) -> Option<String> {
    let char_count = term.chars().count();

    if multi_term && char_count == 1 {
        return None;
    }

    if multi_term && char_count <= 2 {
        Some(term.to_string())
    } else {
        Some(format!("{term}*"))
    }
}

struct SearchDocument {
    item_type: &'static str,
    item_id: i64,
    parent_id: Option<i64>,
    project_id: Option<i64>,
    title: String,
    body: String,
}

async fn insert_search_documents(
    db: &impl ConnectionTrait,
    documents: impl IntoIterator<Item = SearchDocument>,
) -> Result<(), DbErr> {
    let mut chunk = Vec::with_capacity(SEARCH_INSERT_DOCUMENT_LIMIT);
    let mut chunk_body_bytes = 0;

    for document in documents {
        if should_flush_search_document_chunk(chunk.len(), chunk_body_bytes, document.body.len()) {
            insert_search_document_chunk(db, &mut chunk).await?;
            chunk_body_bytes = 0;
        }

        chunk_body_bytes = chunk_body_bytes.saturating_add(document.body.len());
        chunk.push(document);
    }

    if !chunk.is_empty() {
        insert_search_document_chunk(db, &mut chunk).await?;
    }

    Ok(())
}

fn should_flush_search_document_chunk(
    current_documents: usize,
    current_body_bytes: usize,
    next_body_bytes: usize,
) -> bool {
    current_documents > 0
        && (current_documents >= SEARCH_INSERT_DOCUMENT_LIMIT
            || current_body_bytes.saturating_add(next_body_bytes) > SEARCH_INSERT_BODY_BUDGET)
}

async fn insert_search_document_chunk(
    db: &impl ConnectionTrait,
    chunk: &mut Vec<SearchDocument>,
) -> Result<(), DbErr> {
    let placeholders = std::iter::repeat_n("(?, ?, ?, ?, ?, ?)", chunk.len())
        .collect::<Vec<_>>()
        .join(", ");

    let sql = format!(
        "INSERT INTO search_index
                 (item_type, item_id, parent_id, project_id, title, body)
              VALUES {placeholders}"
    );

    let mut values: Vec<Value> = Vec::with_capacity(chunk.len() * 6);

    for doc in chunk.drain(..) {
        values.push(doc.item_type.into());
        values.push(doc.item_id.into());
        values.push(doc.parent_id.into());
        values.push(doc.project_id.into());
        values.push(doc.title.into());
        values.push(doc.body.into());
    }

    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        sql,
        values,
    ))
    .await?;

    Ok(())
}

pub async fn delete_search_item(
    db: &impl ConnectionTrait,
    item_type: &str,
    item_id: u32,
) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "DELETE FROM search_index WHERE item_type = ? AND item_id = ?",
        [
            Value::from(item_type.to_string()),
            Value::from(item_id as i64),
        ],
    ))
    .await?;
    Ok(())
}

pub async fn remove_note_from_index(db: &impl ConnectionTrait, note_id: u32) -> Result<(), DbErr> {
    delete_search_item(db, "note", note_id).await
}

pub async fn remove_project_subtree_from_index(
    db: &impl ConnectionTrait,
    project_id: u32,
) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "DELETE FROM search_index WHERE item_type = 'note' AND item_id IN (SELECT id FROM note WHERE project_id = ?)",
        [Value::from(project_id as i64)],
    ))
    .await?;
    Ok(())
}

pub async fn index_restored_note(db: &impl ConnectionTrait, note_id: u32) -> Result<(), DbErr> {
    delete_search_item(db, "note", note_id).await?;
    if let Some(document) = visible_note_search_document(db, note_id as i64).await? {
        insert_search_documents(db, [document]).await?;
    }
    Ok(())
}

pub async fn index_restored_project_subtree(
    db: &impl ConnectionTrait,
    project_id: u32,
) -> Result<(), DbErr> {
    remove_project_subtree_from_index(db, project_id).await?;
    index_visible_project_subtree(db, project_id as i64).await
}

async fn visible_note_search_document(
    db: &impl ConnectionTrait,
    note_id: i64,
) -> Result<Option<SearchDocument>, DbErr> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT id, project_id, title, cached_content FROM note WHERE id = ? AND deleted_at IS NULL AND (project_id IS NULL OR project_id IN (SELECT id FROM project WHERE deleted_at IS NULL))",
            [Value::from(note_id)],
        ))
        .await?;

    match row {
        Some(row) => Ok(Some(SearchDocument {
            item_type: "note",
            item_id: row.try_get::<i64>("", "id")?,
            parent_id: Some(note_id),
            project_id: row.try_get::<Option<i64>>("", "project_id")?,
            title: row.try_get::<String>("", "title")?,
            body: row.try_get::<String>("", "cached_content")?,
        })),
        None => Ok(None),
    }
}

async fn index_visible_project_subtree(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<(), DbErr> {
    let project = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT id FROM project WHERE id = ? AND deleted_at IS NULL",
            [Value::from(project_id)],
        ))
        .await?;
    if project.is_none() {
        return Ok(());
    }

    let note_rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT id, project_id, title, cached_content FROM note WHERE project_id = ? AND deleted_at IS NULL",
            [Value::from(project_id)],
        ))
        .await?;
    for row in note_rows {
        let id: i64 = row.try_get("", "id")?;
        insert_search_documents(
            db,
            [SearchDocument {
                item_type: "note",
                item_id: id,
                parent_id: Some(id),
                project_id: row.try_get("", "project_id")?,
                title: row.try_get("", "title")?,
                body: row.try_get("", "cached_content")?,
            }],
        )
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        SEARCH_INSERT_BODY_BUDGET, SearchResultKind, fts_query, preview_anchor_terms,
        rebuild_search_index, search_workspace, should_flush_search_document_chunk,
    };
    use crate::test_alloc;
    use anyhow::Result;
    use entity::{note, project};
    use migration::{Migrator, MigratorTrait};
    use sea_orm::{
        ActiveModelTrait, ActiveValue::Set, ConnectionTrait, Database, DbBackend, MockDatabase,
        Value,
    };

    #[test]
    fn search_insert_batches_cap_accumulated_large_bodies() {
        let document_body = SEARCH_INSERT_BODY_BUDGET / 2 + 1;

        assert!(!should_flush_search_document_chunk(0, 0, document_body));
        assert!(should_flush_search_document_chunk(
            1,
            document_body,
            document_body
        ));
        assert!(!should_flush_search_document_chunk(
            0,
            0,
            SEARCH_INSERT_BODY_BUDGET + 1
        ));
    }

    #[tokio::test]
    async fn streamed_rebuild_preserves_search_documents() -> Result<()> {
        let db = Database::connect("sqlite::memory:").await?;
        Migrator::up(&db, None).await?;

        let project = project::ActiveModel {
            name: Set("Search proof".to_string()),
            archived: Set(false),
            position: Set(0),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        note::ActiveModel {
            id: Set(1),
            title: Set("Note title".to_string()),
            project_id: Set(Some(project.id)),
            file_path: Set(None),
            file_managed_by_app: Set(false),
            cached_content: Set("Note body".to_string()),
            file_missing_since: Set(None),
            created_at: Set(1),
            updated_at: Set(1),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        let deleted_project = project::ActiveModel {
            name: Set("Deleted search hierarchy".to_string()),
            archived: Set(false),
            position: Set(1),
            deleted_at: Set(Some(1)),
            ..Default::default()
        }
        .insert(&db)
        .await?;
        note::ActiveModel {
            id: Set(2),
            title: Set("Excluded note".to_string()),
            project_id: Set(Some(deleted_project.id)),
            file_path: Set(None),
            file_managed_by_app: Set(false),
            cached_content: Set("Excluded body".to_string()),
            file_missing_since: Set(None),
            created_at: Set(2),
            updated_at: Set(2),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        rebuild_search_index(&db).await?;

        let rows = db
            .query_all_raw(sea_orm::Statement::from_string(
                sea_orm::DbBackend::Sqlite,
                "SELECT item_type, title, body FROM search_index ORDER BY rowid",
            ))
            .await?;
        let documents = rows
            .into_iter()
            .map(|row| {
                Ok::<_, sea_orm::DbErr>((
                    row.try_get::<String>("", "item_type")?,
                    row.try_get::<String>("", "title")?,
                    row.try_get::<String>("", "body")?,
                ))
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;

        assert_eq!(
            documents,
            vec![(
                "note".to_string(),
                "Note title".to_string(),
                "Note body".to_string(),
            )]
        );

        let results = search_workspace(&db, "Note", 16).await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, SearchResultKind::Note);
        assert_eq!(results[0].item_id, 1);
        assert_eq!(results[0].title, "Note title");

        Ok(())
    }

    #[tokio::test]
    async fn search_workspace_preserves_empty_results_without_catalog_items() -> Result<()> {
        let db = Database::connect("sqlite::memory:").await?;
        Migrator::up(&db, None).await?;

        assert!(search_workspace(&db, "missing", 20).await?.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn search_workspace_skips_catalog_for_empty_results() -> Result<()> {
        let empty_rows = || Vec::<BTreeMap<String, Value>>::new();
        let db = MockDatabase::new(DbBackend::Sqlite)
            .append_query_results([empty_rows(), empty_rows(), empty_rows(), empty_rows()])
            .into_connection();

        assert!(search_workspace(&db, "missing", 20).await?.is_empty());

        assert_eq!(db.into_transaction_log().len(), 1);

        Ok(())
    }

    #[test]
    fn anchor_terms_derive_from_match_query() {
        assert_eq!(
            preview_anchor_terms("the* root* fix* was"),
            vec!["the", "root", "fix", "was"]
        );
        assert_eq!(preview_anchor_terms("needle*"), vec!["needle"]);
    }

    #[tokio::test]
    async fn multi_word_preview_is_anchored_at_term_cooccurrence() -> Result<()> {
        let db = Database::connect("sqlite::memory:").await?;
        Migrator::up(&db, None).await?;

        let filler = "The preliminary note was just filler text. ".repeat(160);
        let body = format!(
            "{filler}\n\n## Why the first root fix was still not enough\n\nThe first repair attempt was incomplete, so the root cause survived the fix."
        );
        assert!(body.len() > 6000);
        note::ActiveModel {
            title: Set("The Bug".to_string()),
            project_id: Set(None),
            file_path: Set(None),
            file_managed_by_app: Set(false),
            cached_content: Set(body),
            file_missing_since: Set(None),
            created_at: Set(1),
            updated_at: Set(1),
            ..Default::default()
        }
        .insert(&db)
        .await?;
        rebuild_search_index(&db).await?;

        let results = search_workspace(&db, "the root fix was", 10).await?;
        assert_eq!(results.len(), 1);
        let plain_preview = results[0].preview.replace(['\u{1}', '\u{2}'], "");
        assert!(
            plain_preview.contains("first root fix"),
            "preview should cover the co-occurrence region, preview starts with: {}",
            plain_preview.chars().take(200).collect::<String>()
        );
        Ok(())
    }

    #[tokio::test]
    async fn multi_word_preview_prefers_late_cluster_over_early_scatter() -> Result<()> {
        let db = Database::connect("sqlite::memory:").await?;
        Migrator::up(&db, None).await?;

        let filler =
            "Restarting Castle fixed everything. The root cause analysis was routine work. "
                .repeat(120);
        let body = format!(
            "{filler}\n\n## Why the first root fix was still not enough\n\nThe first repair attempt was incomplete."
        );
        assert!(body.len() > 9000);
        note::ActiveModel {
            title: Set("The Bug".to_string()),
            project_id: Set(None),
            file_path: Set(None),
            file_managed_by_app: Set(false),
            cached_content: Set(body),
            file_missing_since: Set(None),
            created_at: Set(1),
            updated_at: Set(1),
            ..Default::default()
        }
        .insert(&db)
        .await?;
        rebuild_search_index(&db).await?;

        let results = search_workspace(&db, "root fix was", 10).await?;
        assert_eq!(results.len(), 1);
        let plain_preview = results[0].preview.replace(['\u{1}', '\u{2}'], "");
        assert!(
            plain_preview.contains("first root fix"),
            "preview should cover the late co-occurrence region, preview starts with: {}",
            plain_preview.chars().take(200).collect::<String>()
        );
        Ok(())
    }

    #[tokio::test]
    #[ignore = "performance proof; run explicitly with one test thread"]
    async fn deleted_hierarchy_filter_heap_benchmark() -> Result<()> {
        const EXCLUDED_NOTE_COUNT: usize = 64;
        const BODY_BYTES: usize = 1024 * 1024;

        let db = Database::connect("sqlite::memory:").await?;
        Migrator::up(&db, None).await?;

        let active_project = project::ActiveModel {
            name: Set("Active".to_string()),
            archived: Set(false),
            position: Set(0),
            ..Default::default()
        }
        .insert(&db)
        .await?;
        let deleted_project = project::ActiveModel {
            name: Set("Deleted".to_string()),
            archived: Set(false),
            position: Set(1),
            deleted_at: Set(Some(1)),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        note::ActiveModel {
            id: Set(1),
            title: Set("Indexed note".to_string()),
            project_id: Set(Some(active_project.id)),
            file_path: Set(None),
            file_managed_by_app: Set(false),
            cached_content: Set("active".to_string()),
            file_missing_since: Set(None),
            created_at: Set(0),
            updated_at: Set(0),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        let body = "x".repeat(BODY_BYTES);
        for index in 0..EXCLUDED_NOTE_COUNT {
            note::ActiveModel {
                id: Set(index as i64 + 2),
                title: Set(format!("Excluded note {index}")),
                project_id: Set(Some(deleted_project.id)),
                file_path: Set(None),
                file_managed_by_app: Set(false),
                cached_content: Set(body.clone()),
                file_missing_since: Set(None),
                created_at: Set(index as i64 + 1),
                updated_at: Set(index as i64 + 1),
                ..Default::default()
            }
            .insert(&db)
            .await?;
        }

        drop(body);
        let allocation = test_alloc::start_measurement();
        rebuild_search_index(&db).await?;
        let allocation = allocation.finish();

        let indexed_documents = db
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DbBackend::Sqlite,
                "SELECT COUNT(*) AS count FROM search_index",
            ))
            .await?
            .ok_or_else(|| anyhow::anyhow!("search index count query returned no row"))?
            .try_get::<i64>("", "count")?;

        assert_eq!(indexed_documents, 1);
        println!(
            "excluded_note_body_bytes={} peak_heap_growth_bytes={} total_allocated_bytes={}",
            EXCLUDED_NOTE_COUNT * BODY_BYTES,
            allocation.peak_growth_bytes,
            allocation.allocated_bytes,
        );

        Ok(())
    }

    #[test]
    fn fts_query_splits_hyphenated_terms() {
        assert_eq!(fts_query("edge-case"), Some("edge* case*".to_string()));
    }

    #[test]
    fn fts_query_ignores_repeated_punctuation() {
        assert_eq!(
            fts_query("Rust / GPUI: search"),
            Some("Rust* GPUI* search*".to_string())
        );
    }

    #[test]
    fn fts_query_does_not_prefix_single_letter_terms_in_phrases() {
        assert_eq!(
            fts_query("This is a working"),
            Some("This* is working*".to_string())
        );
    }

    #[test]
    fn fts_query_keeps_single_letter_query_searchable() {
        assert_eq!(fts_query("a"), Some("a*".to_string()));
    }

    #[test]
    fn fts_query_keeps_two_letter_terms_exact_in_phrases() {
        assert_eq!(fts_query("ui state"), Some("ui state*".to_string()));
    }

    #[test]
    fn fts_query_falls_back_to_exact_single_letter_terms() {
        assert_eq!(fts_query("a i"), Some("a i".to_string()));
    }

    #[test]
    fn fts_query_preserves_unicode_term_boundaries() {
        assert_eq!(fts_query("café 東京"), Some("café* 東京".to_string()));
    }

    #[test]
    fn fts_query_rejects_punctuation_only_queries() {
        assert_eq!(fts_query("---"), None);
    }
}
