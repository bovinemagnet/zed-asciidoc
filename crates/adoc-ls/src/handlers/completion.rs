use std::path::Path;

use adoc_antora::AntoraCatalog;

use adoc_core::{canonical_id, Document, SourceRange};
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

/// One candidate per section, plus one per anchor that is not attached to a heading.
///
/// `WorkspaceIndex::anchors_in` deliberately registers several id forms per section — the
/// title, Antora's generated form, Asciidoctor's `_`-prefixed form — so that *resolution* is
/// forgiving about how a reference is written. That makes it the wrong input for
/// *enumeration*: collapsing those forms back down, whether by the location they point at or
/// by a punctuation-insensitive spelling, either merges two distinct sections that happen to
/// normalise the same way (`Set-up` and `Setup`), or, when a section declares an explicit
/// anchor, still offers Asciidoctor's generated id alongside it — an id the explicit anchor
/// actually suppresses, so completion would offer a link that does not resolve.
///
/// Candidates are built from the document itself instead, which already carries exactly one
/// row per section in the author's own spelling: `Section::id` where the author wrote an
/// explicit anchor, and Antora's generated id (`canonical_id`) where they did not — never a
/// suppressed one. Anchors that are not attached to any heading (a `[[bibliography-entry]]` in
/// body text) are offered too, but only once: an anchor immediately above a heading is *also*
/// that section's `id`, so it is skipped here to avoid duplicating the section's own candidate.
fn anchor_candidates(index: &WorkspaceIndex, path: &Path, range: SourceRange) -> Vec<Candidate> {
    let Some(document) = index.file(path).map(|entry| &entry.document) else {
        return Vec::new();
    };

    let mut candidates: Vec<Candidate> = document
        .sections
        .iter()
        .map(|section| {
            section
                .id
                .clone()
                .unwrap_or_else(|| canonical_id(&section.title))
        })
        .map(|label| candidate(label, range))
        .collect();

    candidates.extend(
        document
            .anchors
            .iter()
            .filter(|anchor| {
                !document
                    .sections
                    .iter()
                    .any(|section| section.id.as_deref() == Some(anchor.id.as_str()))
            })
            .map(|anchor| candidate(anchor.id.clone(), range)),
    );

    candidates.sort_by(|a, b| a.label.cmp(&b.label));
    candidates
}

fn candidate(label: String, range: SourceRange) -> Candidate {
    let sort_text = format!("0{label}");
    Candidate {
        label,
        detail: None,
        sort_text,
        kind: CandidateKind::Anchor,
        range,
    }
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
    fn offers_distinct_candidates_for_sections_that_normalise_to_the_same_alphanumeric_form() {
        let path = Path::new("/docs/guide.adoc");
        let text = "== Set-up\n\n== Setup\n\nSee <<";
        let mut index = WorkspaceIndex::new();
        index.index_source(path, text);
        let document = parse("file:///docs/guide.adoc", text).document;

        let labels: Vec<_> =
            completion_at_offset(&index, &AntoraCatalog::new(), path, &document, text.len())
                .into_iter()
                .map(|candidate| candidate.label)
                .collect();

        assert_eq!(labels, vec!["set-up".to_owned(), "setup".to_owned()]);
    }

    #[test]
    fn an_explicit_anchor_suppresses_the_generated_id() {
        let path = Path::new("/docs/guide.adoc");
        let text = "[[install-linux]]\n== Installing on Linux\n\nSee <<";
        let mut index = WorkspaceIndex::new();
        index.index_source(path, text);
        let document = parse("file:///docs/guide.adoc", text).document;

        let labels: Vec<_> =
            completion_at_offset(&index, &AntoraCatalog::new(), path, &document, text.len())
                .into_iter()
                .map(|candidate| candidate.label)
                .collect();

        assert_eq!(labels, vec!["install-linux".to_owned()]);
    }

    #[test]
    fn offers_a_standalone_anchor_not_attached_to_any_heading() {
        let path = Path::new("/docs/guide.adoc");
        let text = "[[bibliography-entry]]\nSome reference text.\n\nSee <<";
        let mut index = WorkspaceIndex::new();
        index.index_source(path, text);
        let document = parse("file:///docs/guide.adoc", text).document;

        let labels: Vec<_> =
            completion_at_offset(&index, &AntoraCatalog::new(), path, &document, text.len())
                .into_iter()
                .map(|candidate| candidate.label)
                .collect();

        assert!(labels.contains(&"bibliography-entry".to_owned()));
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
