use std::path::Path;

use adoc_antora::AntoraCatalog;
use std::collections::BTreeMap;

use adoc_core::{alphanumeric_id, Document, SourceRange};
use adoc_index::WorkspaceIndex;
use adoc_parser::{completion_context, CompletionKind};

use crate::handlers::definition::reference_target_path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateKind {
    Page,
    Resource,
    Family,
    Directory,
    Anchor,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Candidate {
    /// Inserted verbatim over `range` when the candidate is accepted.
    pub label: String,
    pub detail: Option<String>,
    pub sort_text: String,
    pub kind: CandidateKind,
    pub range: SourceRange,
}

/// Candidates for the construct the cursor sits inside, or an empty list.
///
/// Never returns an error: an unresolvable context, an unknown target and an unreadable
/// directory all mean "no suggestions", which matches how navigation and diagnostics
/// behave here.
#[must_use]
pub fn completion_at_offset(
    index: &WorkspaceIndex,
    antora: &AntoraCatalog,
    current_path: &Path,
    document: &Document,
    offset: usize,
) -> Vec<Candidate> {
    let Some(context) = completion_context(&document.text, offset) else {
        return Vec::new();
    };

    let candidates = match &context.kind {
        CompletionKind::LocalAnchor => anchor_candidates(index, current_path, context.range),
        CompletionKind::XrefAnchor { target } => {
            match reference_target_path(index, antora, current_path, target) {
                Some(path) => anchor_candidates(index, &path, context.range),
                None => Vec::new(),
            }
        }
        // Filled in by later tasks.
        CompletionKind::XrefTarget
        | CompletionKind::IncludeTarget
        | CompletionKind::ImageTarget => Vec::new(),
    };

    filter_by_prefix(candidates, &context.prefix)
}

/// One candidate per section, not one per id form.
///
/// The index registers every form `Section::implicit_ids` produces — the title, Antora's
/// `getting-started` form, Asciidoctor's `_getting_started` form — plus the explicit anchor
/// where a section declares one, so that resolution is forgiving about how a reference is
/// written. An explicit anchor sits on its own line above the heading, so it points at a
/// different `SourceRange` than the heading itself even though it names the same section;
/// grouping by location would therefore leave the explicit form and the generated forms as
/// separate candidates. `alphanumeric_id` is the same punctuation-insensitive comparison
/// `WorkspaceIndex::resolve_anchor` already falls back to, so grouping by it collapses every
/// form of one section together regardless of which line it was declared on. One id per group
/// is then chosen: no leading `_`, then the lowercase form, then lexicographic order. That
/// yields the explicit anchor where one is declared, and Antora's generated id where none is.
fn anchor_candidates(index: &WorkspaceIndex, path: &Path, range: SourceRange) -> Vec<Candidate> {
    let mut by_section: BTreeMap<String, &str> = BTreeMap::new();
    for (id, _location) in index.anchors_in(path) {
        by_section
            .entry(alphanumeric_id(id))
            .and_modify(|chosen| {
                if preference(id) < preference(chosen) {
                    *chosen = id;
                }
            })
            .or_insert(id);
    }

    by_section
        .into_values()
        .map(|id| Candidate {
            label: id.to_owned(),
            detail: None,
            sort_text: format!("0{id}"),
            kind: CandidateKind::Anchor,
            range,
        })
        .collect()
}

/// Lower sorts better. Underscore-prefixed forms last, mixed case next, then shortest.
fn preference(id: &str) -> (bool, bool, &str) {
    (id.starts_with('_'), id.chars().any(char::is_uppercase), id)
}

/// Narrow the list to what the author has typed, case-insensitively and by substring.
///
/// The response is marked incomplete, so the client asks again as the prefix grows and
/// this runs against the longer prefix each time.
fn filter_by_prefix(candidates: Vec<Candidate>, prefix: &str) -> Vec<Candidate> {
    if prefix.is_empty() {
        return candidates;
    }
    let needle = prefix.to_lowercase();
    candidates
        .into_iter()
        .filter(|candidate| candidate.label.to_lowercase().contains(&needle))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use adoc_antora::AntoraCatalog;
    use adoc_index::WorkspaceIndex;
    use adoc_parser::parse;

    use super::completion_at_offset;

    #[test]
    fn completes_anchors_declared_in_the_current_file() {
        let path = Path::new("/docs/guide.adoc");
        let text = "[[intro]]\n== Intro\n\n[[detail]]\n== Detail\n\nSee <<";
        let mut index = WorkspaceIndex::new();
        index.index_source(path, text);
        let document = parse("file:///docs/guide.adoc", text).document;

        let labels: Vec<_> =
            completion_at_offset(&index, &AntoraCatalog::new(), path, &document, text.len())
                .into_iter()
                .map(|candidate| candidate.label)
                .collect();

        assert!(labels.contains(&"intro".to_owned()));
        assert!(labels.contains(&"detail".to_owned()));
    }

    #[test]
    fn offers_one_candidate_per_section_not_one_per_id_form() {
        // The index registers `Detail`, `detail`, `_detail` and `detail` again so that
        // resolution is forgiving. Completion must collapse them to the anchor as written.
        let path = Path::new("/docs/guide.adoc");
        let text = "[[intro]]\n== Intro\n\n[[detail]]\n== Detail\n\nSee <<";
        let mut index = WorkspaceIndex::new();
        index.index_source(path, text);
        let document = parse("file:///docs/guide.adoc", text).document;

        let labels: Vec<_> =
            completion_at_offset(&index, &AntoraCatalog::new(), path, &document, text.len())
                .into_iter()
                .map(|candidate| candidate.label)
                .collect();

        assert_eq!(labels, vec!["detail".to_owned(), "intro".to_owned()]);
    }

    #[test]
    fn prefers_the_generated_id_for_a_section_with_no_explicit_anchor() {
        let path = Path::new("/docs/guide.adoc");
        let text = "== Getting Started\n\nSee <<";
        let mut index = WorkspaceIndex::new();
        index.index_source(path, text);
        let document = parse("file:///docs/guide.adoc", text).document;

        let labels: Vec<_> =
            completion_at_offset(&index, &AntoraCatalog::new(), path, &document, text.len())
                .into_iter()
                .map(|candidate| candidate.label)
                .collect();

        assert_eq!(labels, vec!["getting-started".to_owned()]);
    }

    #[test]
    fn filters_anchors_by_what_has_been_typed() {
        let path = Path::new("/docs/guide.adoc");
        let text = "[[intro]]\n== Intro\n\n[[detail]]\n== Detail\n\nSee <<det";
        let mut index = WorkspaceIndex::new();
        index.index_source(path, text);
        let document = parse("file:///docs/guide.adoc", text).document;

        let labels: Vec<_> =
            completion_at_offset(&index, &AntoraCatalog::new(), path, &document, text.len())
                .into_iter()
                .map(|candidate| candidate.label)
                .collect();

        assert_eq!(labels, vec!["detail".to_owned()]);
    }

    #[test]
    fn completes_anchors_of_another_file_after_a_hash() {
        let index_path = Path::new("/docs/index.adoc");
        let other_path = Path::new("/docs/other.adoc");
        let text = "= Index\n\nSee xref:other.adoc#";
        let mut index = WorkspaceIndex::new();
        index.index_source(index_path, text);
        index.index_source(other_path, "[[details]]\n== Details\n");
        let document = parse("file:///docs/index.adoc", text).document;

        let labels: Vec<_> = completion_at_offset(
            &index,
            &AntoraCatalog::new(),
            index_path,
            &document,
            text.len(),
        )
        .into_iter()
        .map(|candidate| candidate.label)
        .collect();

        assert!(labels.contains(&"details".to_owned()));
    }

    #[test]
    fn returns_nothing_where_there_is_no_context() {
        let path = Path::new("/docs/guide.adoc");
        let text = "= Guide\n\nOrdinary prose.\n";
        let mut index = WorkspaceIndex::new();
        index.index_source(path, text);
        let document = parse("file:///docs/guide.adoc", text).document;

        assert!(
            completion_at_offset(&index, &AntoraCatalog::new(), path, &document, text.len())
                .is_empty()
        );
    }
}
