#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CreateProjectInput {
    #[cfg_attr(
        feature = "schema",
        schemars(description = "Human-readable project name")
    )]
    pub name: String,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RenameProjectInput {
    pub project_id: i64,
    pub name: String,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProjectNotesInput {
    #[cfg_attr(
        feature = "schema",
        schemars(description = "Filter by project ID; omit to include every active note")
    )]
    pub project_id: Option<i64>,
    #[cfg_attr(
        feature = "schema",
        schemars(description = "Maximum results, from 1 to 100; defaults to 50")
    )]
    pub limit: Option<u64>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct NoteInput {
    pub note_id: i64,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SearchNotesInput {
    #[cfg_attr(
        feature = "schema",
        schemars(description = "Case-insensitive text matched against note titles and content")
    )]
    pub query: String,
    pub project_id: Option<i64>,
    #[cfg_attr(
        feature = "schema",
        schemars(description = "Maximum results, from 1 to 100; defaults to 25")
    )]
    pub limit: Option<u64>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CreateNoteInput {
    pub title: String,
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(
        feature = "schema",
        schemars(description = "Initial Markdown or plain-text content")
    )]
    pub content: String,
    #[cfg_attr(
        feature = "schema",
        schemars(description = "Parent project ID; omit for a standalone note")
    )]
    pub project_id: Option<i64>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct UpdateNoteInput {
    pub note_id: i64,
    #[cfg_attr(
        feature = "schema",
        schemars(description = "Replacement title; omit to keep the current title")
    )]
    pub title: Option<String>,
    #[cfg_attr(
        feature = "schema",
        schemars(description = "Replacement content; omit to keep the current content")
    )]
    pub content: Option<String>,
    pub is_pinned: Option<bool>,
    #[cfg_attr(
        feature = "schema",
        schemars(
            description = "Reject the update if the note changed since this updated_at value"
        )
    )]
    pub expected_updated_at: Option<i64>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct MoveNoteInput {
    pub note_id: i64,
    #[cfg_attr(
        feature = "schema",
        schemars(description = "Destination project ID; omit to make the note standalone")
    )]
    pub project_id: Option<i64>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProjectSummary {
    pub id: i64,
    pub name: String,
    pub position: i32,
    pub note_count: u64,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct NoteSummary {
    pub id: i64,
    pub title: String,
    pub project_id: Option<i64>,
    pub project_name: Option<String>,
    pub is_pinned: bool,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct NoteDetail {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub project_id: Option<i64>,
    pub project_name: Option<String>,
    pub file_path: Option<String>,
    pub file_managed_by_app: bool,
    pub file_missing: bool,
    pub is_pinned: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub related_items: Vec<RelatedItemDetail>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RelatedItemDetail {
    pub kind: String,
    pub id: i64,
    pub title: String,
    pub breadcrumb: String,
    pub stable_link: String,
    pub origins: Vec<String>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct NoteLinksDetail {
    pub inbound: Vec<NoteLinkDetail>,
    pub outbound: Vec<NoteLinkDetail>,
    pub unresolved: Vec<NoteLinkDetail>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct NoteLinkDetail {
    pub source_note_id: i64,
    pub source_title: String,
    pub source_project_name: Option<String>,
    pub target_note_id: Option<i64>,
    pub target_title: Option<String>,
    pub target_project_name: Option<String>,
    pub target_kind: Option<String>,
    pub raw_target: String,
    pub display_text: Option<String>,
    pub start_byte: usize,
    pub end_byte: usize,
    pub line_number: usize,
}
