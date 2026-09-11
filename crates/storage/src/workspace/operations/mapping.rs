use super::*;

pub(super) fn note_summary(note: note::Model, projects: &HashMap<i64, String>) -> NoteSummary {
    NoteSummary {
        id: note.id,
        title: note.title,
        project_id: note.project_id,
        project_name: note
            .project_id
            .and_then(|project_id| projects.get(&project_id).cloned()),
        is_pinned: note.is_pinned,
        updated_at: note.updated_at,
    }
}

pub(super) fn note_link_detail(link: crate::note::links::NoteLinkReference) -> NoteLinkDetail {
    NoteLinkDetail {
        source_note_id: link.source_note_id,
        source_title: link.source_title,
        source_project_name: link.source_project_name,
        target_note_id: link.target_note_id,
        target_title: link.target_title,
        target_project_name: link.target_project_name,
        target_kind: None,
        raw_target: link.raw_target,
        display_text: link.display_text,
        start_byte: link.start_byte,
        end_byte: link.end_byte,
        line_number: link.line_number,
    }
}

pub(super) fn unresolved_link_detail(
    link: crate::note::links::UnresolvedLinkReference,
) -> NoteLinkDetail {
    NoteLinkDetail {
        source_note_id: link.source_note_id,
        source_title: link.source_title,
        source_project_name: link.source_project_name,
        target_note_id: None,
        target_title: None,
        target_project_name: None,
        target_kind: link.target_kind.map(|kind| kind.as_str().to_string()),
        raw_target: link.raw_target,
        display_text: link.display_text,
        start_byte: link.start_byte,
        end_byte: link.end_byte,
        line_number: link.line_number,
    }
}

pub(super) fn workspace_origin_label(
    origin: crate::workspace::links::WorkspaceLinkOrigin,
) -> &'static str {
    match origin {
        crate::workspace::links::WorkspaceLinkOrigin::Manual => "manual",
        crate::workspace::links::WorkspaceLinkOrigin::Wikilink => "wikilink",
        crate::workspace::links::WorkspaceLinkOrigin::Embed => "embed",
    }
}

pub(super) fn related_item_detail(
    entry: crate::workspace::links::WorkspaceCatalogEntry,
    origins: Vec<String>,
) -> RelatedItemDetail {
    RelatedItemDetail {
        kind: entry.item.kind.as_str().to_string(),
        id: entry.item.id,
        title: entry.title.clone(),
        breadcrumb: entry.breadcrumb(),
        stable_link: entry.stable_link(),
        origins,
    }
}

