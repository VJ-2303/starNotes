use std::fmt;

use super::{WorkspaceCatalogEntry, WorkspaceItemKind, WorkspaceItemRef};

/// The immutable snapshot consumed by completion, indexing, and preview.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorkspaceReferenceCatalog {
    pub items: Vec<WorkspaceCatalogEntry>,
    pub aliases: Vec<WorkspaceReferenceAlias>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceReferenceAlias {
    pub target: WorkspaceAliasTarget,
    pub alias: String,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum WorkspaceAliasTarget {
    Project(i64),
    Item(WorkspaceItemRef),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceReferencePath {
    pub kind: WorkspaceItemKind,
    pub segments: Vec<String>,
    pub display_text: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResolvedWorkspaceReference {
    Item(WorkspaceItemRef),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceReferenceResolveError {
    Invalid,
    Missing,
    Ambiguous,
}

impl fmt::Display for WorkspaceReferenceResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Invalid => "invalid workspace reference",
            Self::Missing => "workspace reference was not found",
            Self::Ambiguous => "workspace reference is ambiguous",
        })
    }
}

impl std::error::Error for WorkspaceReferenceResolveError {}

impl WorkspaceReferenceCatalog {
    pub fn item(&self, item: WorkspaceItemRef) -> Option<&WorkspaceCatalogEntry> {
        self.items.iter().find(|entry| entry.item == item)
    }

    pub fn item_path(&self, entry: &WorkspaceCatalogEntry) -> Vec<String> {
        entry.path_segments()
    }

    pub fn format_item_link(
        &self,
        item: WorkspaceItemRef,
        display_text: Option<&str>,
    ) -> Option<String> {
        let entry = self.item(item)?;
        let path = self.shortest_unique_path(entry);
        let mut formatted = format!("[[{}:{}]]", item.kind.as_str(), format_segments(&path));
        if let Some(display_text) = display_text.filter(|text| !text.trim().is_empty()) {
            formatted.insert_str(
                formatted.len() - 2,
                &format!("|{}", escape_segment(display_text.trim())),
            );
        }
        Some(formatted)
    }

    fn shortest_unique_path(&self, entry: &WorkspaceCatalogEntry) -> Vec<String> {
        let full = self.item_path(entry);
        for start in (0..full.len()).rev() {
            let candidate = &full[start..];
            let matches = self
                .items
                .iter()
                .filter(|other| other.item.kind == entry.item.kind)
                .filter(|other| path_matches_suffix(self, other, candidate))
                .count();
            if matches == 1 {
                return candidate.to_vec();
            }
        }
        full
    }
}

pub fn parse_reference_target(raw_target: &str) -> Option<WorkspaceReferencePath> {
    let (prefix, value) = raw_target.trim().split_once(':')?;
    let kind = WorkspaceItemKind::parse(prefix.trim())?;
    if has_unterminated_escape(value) || contains_unescaped_bracket(value) {
        return None;
    }
    let (value, display_text) = split_unescaped(value, '|')
        .map(|(target, display)| {
            if contains_unescaped(display, '|')
                || contains_unescaped(display, '#')
                || contains_unescaped(display, '[')
                || contains_unescaped(display, ']')
            {
                return ("", None);
            }
            (
                target,
                Some(unescape_segment(display.trim())).filter(|display| !display.is_empty()),
            )
        })
        .unwrap_or((value, None));
    if value.is_empty() || contains_unescaped(value, '#') {
        return None;
    }
    let segments = split_segments(value).ok()?;
    (!segments.is_empty()).then_some(WorkspaceReferencePath {
        kind,
        segments,
        display_text,
    })
}

pub fn resolve_reference(
    reference: &WorkspaceReferencePath,
    catalog: &WorkspaceReferenceCatalog,
) -> Result<ResolvedWorkspaceReference, WorkspaceReferenceResolveError> {
    let candidates = catalog
        .items
        .iter()
        .filter(|entry| entry.item.kind == reference.kind)
        .filter(|entry| path_matches_suffix(catalog, entry, &reference.segments))
        .collect::<Vec<_>>();
    match candidates.as_slice() {
        [entry] => Ok(ResolvedWorkspaceReference::Item(entry.item)),
        [] => Err(WorkspaceReferenceResolveError::Missing),
        _ => Err(WorkspaceReferenceResolveError::Ambiguous),
    }
}

pub fn resolve_reference_target(
    raw_target: &str,
    catalog: &WorkspaceReferenceCatalog,
) -> Result<ResolvedWorkspaceReference, WorkspaceReferenceResolveError> {
    let reference =
        parse_reference_target(raw_target).ok_or(WorkspaceReferenceResolveError::Invalid)?;
    resolve_reference(&reference, catalog)
}

/// Resolve a readable item reference and return the catalog entry that owns the target.
pub fn resolve_reference_entry<'a>(
    raw_target: &str,
    catalog: &'a WorkspaceReferenceCatalog,
) -> Result<&'a WorkspaceCatalogEntry, WorkspaceReferenceResolveError> {
    let item = match resolve_reference_target(raw_target, catalog)? {
        ResolvedWorkspaceReference::Item(item) => item,
    };
    catalog
        .item(item)
        .ok_or(WorkspaceReferenceResolveError::Missing)
}

fn path_matches_suffix(
    catalog: &WorkspaceReferenceCatalog,
    entry: &WorkspaceCatalogEntry,
    candidate: &[String],
) -> bool {
    let options = path_options(catalog, entry);
    if candidate.len() > options.len() {
        return false;
    }
    options[options.len() - candidate.len()..]
        .iter()
        .zip(candidate)
        .all(|(segment_options, requested)| {
            segment_options
                .iter()
                .any(|option| normalize(option) == normalize(requested))
        })
}

fn path_options(
    catalog: &WorkspaceReferenceCatalog,
    entry: &WorkspaceCatalogEntry,
) -> Vec<Vec<String>> {
    let mut options = Vec::new();
    if let (Some(project_id), Some(project_name)) = (entry.project_id, entry.project_name.as_ref())
    {
        options.push(alias_options(
            catalog,
            WorkspaceAliasTarget::Project(project_id),
            project_name,
        ));
    }
    options.push(alias_options(
        catalog,
        WorkspaceAliasTarget::Item(entry.item),
        &entry.title,
    ));
    options
}

fn alias_options(
    catalog: &WorkspaceReferenceCatalog,
    target: WorkspaceAliasTarget,
    current: &str,
) -> Vec<String> {
    let mut options = vec![current.to_string()];
    options.extend(
        catalog
            .aliases
            .iter()
            .filter(|alias| alias.target == target)
            .map(|alias| alias.alias.clone()),
    );
    options
}

fn split_segments(value: &str) -> Result<Vec<String>, ()> {
    let raw_segments = split_unescaped_all(value, '/')?;
    if raw_segments.iter().any(|segment| segment.trim().is_empty()) {
        return Err(());
    }
    let segments = raw_segments
        .into_iter()
        .map(|segment| unescape_segment(segment.trim()))
        .collect::<Vec<_>>();
    (!segments.is_empty()).then_some(segments).ok_or(())
}

fn split_unescaped(value: &str, delimiter: char) -> Option<(&str, &str)> {
    let mut escaped = false;
    for (index, character) in value.char_indices() {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == delimiter {
            return Some((&value[..index], &value[index + character.len_utf8()..]));
        }
    }
    None
}

fn split_unescaped_all(value: &str, delimiter: char) -> Result<Vec<&str>, ()> {
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut escaped = false;
    for (index, character) in value.char_indices() {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == delimiter {
            segments.push(&value[start..index]);
            start = index + character.len_utf8();
        }
    }
    if escaped {
        return Err(());
    }
    segments.push(&value[start..]);
    Ok(segments)
}

pub fn escape_segment(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if matches!(character, '\\' | '/' | '#' | '|' | '[' | ']') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

pub fn unescape_segment(value: &str) -> String {
    let mut unescaped = String::with_capacity(value.len());
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            unescaped.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            unescaped.push(character);
        }
    }
    if escaped {
        unescaped.push('\\');
    }
    unescaped
}

fn has_unterminated_escape(value: &str) -> bool {
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        }
    }
    escaped
}

fn contains_unescaped_bracket(value: &str) -> bool {
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if matches!(character, '[' | ']') {
            return true;
        }
    }
    false
}

fn contains_unescaped(value: &str, delimiter: char) -> bool {
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == delimiter {
            return true;
        }
    }
    false
}

fn format_segments(segments: &[String]) -> String {
    segments
        .iter()
        .map(|segment| escape_segment(segment))
        .collect::<Vec<_>>()
        .join(" / ")
}

fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
}

impl WorkspaceItemKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "note" => Some(Self::Note),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_readable_note_targets() {
        let parsed =
            parse_reference_target(r"note:Castle / Main\#Note|Dashboard\|label")
                .expect("readable note target should parse");
        assert_eq!(parsed.kind, WorkspaceItemKind::Note);
        assert_eq!(parsed.segments, vec!["Castle", "Main#Note"]);
        assert_eq!(parsed.display_text.as_deref(), Some("Dashboard|label"));

        let escaped = parse_reference_target(r"note:Launch\/v2|Open \[note\]")
            .expect("escaped note reference should parse");
        assert_eq!(escaped.segments, vec!["Launch/v2"]);
        assert_eq!(escaped.display_text.as_deref(), Some("Open [note]"));
    }

    #[test]
    fn formatter_uses_the_shortest_unique_hierarchy_and_escapes_delimiters() {
        let catalog = WorkspaceReferenceCatalog {
            items: vec![
                WorkspaceCatalogEntry {
                    item: WorkspaceItemRef {
                        kind: WorkspaceItemKind::Note,
                        id: 1,
                    },
                    title: "Road/Map".into(),
                    project_id: Some(10),
                    project_name: Some("Castle".into()),
                },
                WorkspaceCatalogEntry {
                    item: WorkspaceItemRef {
                        kind: WorkspaceItemKind::Note,
                        id: 2,
                    },
                    title: "Road/Map".into(),
                    project_id: Some(11),
                    project_name: Some("Other".into()),
                },
            ],
            ..Default::default()
        };
        assert_eq!(
            catalog.format_item_link(
                WorkspaceItemRef {
                    kind: WorkspaceItemKind::Note,
                    id: 1,
                },
                Some("Open | now"),
            ),
            Some("[[note:Castle / Road\\/Map|Open \\| now]]".into())
        );
    }

    #[test]
    fn malformed_references_are_rejected_and_matching_is_case_insensitive() {
        assert!(parse_reference_target("note:Roadmap / ").is_none());
        assert!(parse_reference_target("note:Roadmap#").is_none());
        assert!(parse_reference_target("note:Roadmap#Current#Other").is_none());
        assert!(parse_reference_target("note:Roadmap|One|Two").is_none());
        assert!(parse_reference_target(r"note:Roadmap\").is_none());
        assert!(parse_reference_target("unknown:Roadmap").is_none());

        let catalog = WorkspaceReferenceCatalog {
            items: vec![WorkspaceCatalogEntry {
                item: WorkspaceItemRef {
                    kind: WorkspaceItemKind::Note,
                    id: 4,
                },
                title: "Roadmap".into(),
                project_id: None,
                project_name: None,
            }],
            ..Default::default()
        };
        assert_eq!(
            resolve_reference_target("NOTE:roadMAP", &catalog),
            Ok(ResolvedWorkspaceReference::Item(WorkspaceItemRef {
                kind: WorkspaceItemKind::Note,
                id: 4,
            }))
        );
    }

    #[test]
    fn aliases_resolve_hierarchy_levels() {
        let note = WorkspaceItemRef {
            kind: WorkspaceItemKind::Note,
            id: 4,
        };
        let catalog = WorkspaceReferenceCatalog {
            items: vec![
                WorkspaceCatalogEntry {
                    item: note,
                    title: "Roadmap".into(),
                    project_id: Some(1),
                    project_name: Some("Castle".into()),
                },
            ],
            aliases: vec![
                WorkspaceReferenceAlias {
                    target: WorkspaceAliasTarget::Item(note),
                    alias: "Old note".into(),
                },
            ],
        };

        assert_eq!(
            resolve_reference_target("note:Old note", &catalog),
            Ok(ResolvedWorkspaceReference::Item(note))
        );
    }
}
