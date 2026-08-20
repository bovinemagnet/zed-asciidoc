# Completion for References and Includes — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give `adoc-ls` a `textDocument/completion` provider that completes `xref:` targets, `include::` targets, `image:` targets, and anchors, both inside an Antora module and in a plain AsciiDoc workspace.

**Architecture:** A new pure function in `adoc-parser` detects what the cursor is inside from half-typed text. `adoc-antora` and `adoc-index` gain ordered-range enumerators so candidates cost what the answer costs rather than what the workspace costs. A new `adoc-ls/src/handlers/completion.rs` assembles candidates from those enumerators, and `protocol.rs` converts them to `lsp-types`. No new crate; the Zed extension is untouched.

**Tech Stack:** Rust 2021, `lsp-server` + `lsp-types` (pinned), `BTreeMap` ranges, no new dependencies.

**Spec:** `docs/prd/DESIGN-Completion_for_References_and_Includes.md`

## Global Constraints

Copied verbatim from `CLAUDE.md` and the spec. Every task's requirements implicitly include these.

- `unsafe_code = "forbid"` workspace-wide; avoid global mutable state.
- Never leak `lsp-types` or transport types into `adoc-core`, `adoc-parser`, `adoc-index`, or `adoc-antora`.
- Lookups go through direct maps; **never add features that scan every document**. Every enumerator in this plan is a `BTreeMap` range plus `take_while`.
- Diagnostics and navigation stay conservative. For completion this means: **every failure path returns an empty list, never an LSP error and never a guess.**
- Incomplete AsciiDoc is normal input — nothing here may panic on half-typed syntax.
- Use `Path`/`PathBuf` and the crate's `normalize_path`; do not canonicalise paths that may not exist.
- No new external dependencies. Any that were added would need exact pinning (`=1.0.229` style).
- Always pass `--workspace` to cargo; `default-members = ["."]` means a bare `cargo test` covers only the extension crate.
- British spelling in prose and documentation.
- Author is Paul Snow; version 0.0.0 where either is required.
- Tests live in `#[cfg(test)] mod tests` beside the code. Fixtures are reached with `Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/...")` — never absolute developer-machine paths.
- Baseline before calling any task done:
  ```sh
  cargo fmt --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  ```

## File Structure

| File | Responsibility |
|---|---|
| `crates/adoc-parser/src/completion.rs` | **Create.** `completion_context` and its types. Detects what the cursor is inside. Pure: `&str` + offset in, `Option<CompletionContext>` out. |
| `crates/adoc-parser/src/line_parser.rs` | **Modify.** Make `is_boundary` `pub(crate)`; move `COMMENT_DELIMITER` here from `parser.rs` so both scanners share one definition. |
| `crates/adoc-parser/src/parser.rs` | **Modify.** Import `COMMENT_DELIMITER` from `line_parser` instead of declaring it. |
| `crates/adoc-parser/src/lib.rs` | **Modify.** `mod completion;` and re-export its three public items. |
| `crates/adoc-antora/src/catalog.rs` | **Modify.** Add `resources_in` and `modules_of` range enumerators. |
| `crates/adoc-index/src/index.rs` | **Modify.** Add `anchors_in` and `files_under` range enumerators. |
| `crates/adoc-index/src/workspace.rs` | **Modify.** Add `list_directory` and `DirectoryEntry` — the only filesystem read on the completion path. |
| `crates/adoc-index/src/lib.rs` | **Modify.** Re-export `DirectoryEntry` and `list_directory`. |
| `crates/adoc-ls/src/handlers/definition.rs` | **Modify.** Add `reference_target_path`, so anchor completion resolves a target through the same order Go to Definition uses. |
| `crates/adoc-ls/src/handlers/completion.rs` | **Create.** Candidate assembly. Editor-agnostic: byte offsets in, `Vec<Candidate>` out. |
| `crates/adoc-ls/src/handlers/mod.rs` | **Modify.** `pub mod completion;` |
| `crates/adoc-ls/src/capabilities.rs` | **Modify.** Advertise `completion_provider`. |
| `crates/adoc-ls/src/protocol.rs` | **Modify.** One dispatch arm plus `completion_response`. |
| `crates/adoc-ls/tests/stdio.rs` | **Modify.** A real `textDocument/completion` round-trip. |
| `README.md` | **Modify.** Move completion out of "Not implemented yet". |

Tasks 1–4 are independent of each other and may be done in any order. Task 5 needs 1 and 4. Tasks 6 and 7 need 5.

---

### Task 1: Context detection in `adoc-parser`

**Files:**
- Create: `crates/adoc-parser/src/completion.rs`
- Modify: `crates/adoc-parser/src/line_parser.rs` (`is_boundary` visibility, `COMMENT_DELIMITER`)
- Modify: `crates/adoc-parser/src/parser.rs:36-37` (use the moved constant)
- Modify: `crates/adoc-parser/src/lib.rs`
- Test: `crates/adoc-parser/src/completion.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `adoc_core::SourceRange`; crate-private `line_parser::{content, is_verbatim_delimiter, is_boundary}`.
- Produces:
  ```rust
  pub fn completion_context(text: &str, offset: usize) -> Option<CompletionContext>
  pub struct CompletionContext { pub kind: CompletionKind, pub prefix: String, pub range: SourceRange }
  pub enum CompletionKind { XrefTarget, XrefAnchor { target: String }, IncludeTarget, ImageTarget, LocalAnchor }
  ```

**Background:** `parser.rs` records only *complete* macros — `parser::tests::tolerates_incomplete_constructs` asserts `xref:unfinished[` yields nothing. Completion therefore cannot reuse `document.references` and needs this separate scanner. `range` runs from the start of the target to the cursor, so an accepted completion replaces exactly what was typed.

- [ ] **Step 1: Write the failing tests for the five kinds**

Create `crates/adoc-parser/src/completion.rs` containing only the test module for now:

```rust
#[cfg(test)]
mod tests {
    use adoc_core::SourceRange;

    use super::{completion_context, CompletionKind};

    #[test]
    fn detects_an_xref_target() {
        let text = "= Demo\n\nSee xref:get";
        let context = completion_context(text, text.len()).expect("a context");

        assert_eq!(context.kind, CompletionKind::XrefTarget);
        assert_eq!(context.prefix, "get");
        assert_eq!(context.range, SourceRange::new(text.len() - 3, text.len()));
    }

    #[test]
    fn detects_an_anchor_after_a_hash() {
        let text = "See xref:other.adoc#det";
        let context = completion_context(text, text.len()).expect("a context");

        assert_eq!(
            context.kind,
            CompletionKind::XrefAnchor {
                target: "other.adoc".to_owned()
            }
        );
        assert_eq!(context.prefix, "det");
    }

    #[test]
    fn treats_a_leading_hash_as_an_anchor_in_this_file() {
        let text = "See xref:#det";
        let context = completion_context(text, text.len()).expect("a context");

        assert_eq!(
            context.kind,
            CompletionKind::XrefAnchor {
                target: String::new()
            }
        );
        assert_eq!(context.prefix, "det");
    }

    #[test]
    fn detects_an_include_target() {
        let text = "include::partial$no";
        let context = completion_context(text, text.len()).expect("a context");

        assert_eq!(context.kind, CompletionKind::IncludeTarget);
        assert_eq!(context.prefix, "partial$no");
    }

    #[test]
    fn detects_block_and_inline_image_targets() {
        for text in ["image::arch", "Shown image:arch"] {
            let context = completion_context(text, text.len()).expect("a context");

            assert_eq!(context.kind, CompletionKind::ImageTarget);
            assert_eq!(context.prefix, "arch");
        }
    }

    #[test]
    fn detects_a_local_anchor() {
        let text = "See <<int";
        let context = completion_context(text, text.len()).expect("a context");

        assert_eq!(context.kind, CompletionKind::LocalAnchor);
        assert_eq!(context.prefix, "int");
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p adoc-parser completion`
Expected: compile failure — `completion_context` is not defined, and `completion.rs` is not yet a module.

- [ ] **Step 3: Expose the two crate-private helpers**

In `crates/adoc-parser/src/line_parser.rs`, move the constant in from `parser.rs` and widen `is_boundary`:

```rust
/// Comment blocks are the one delimited block Asciidoctor does not expand includes in.
pub(crate) const COMMENT_DELIMITER: &str = "////";
```

```rust
pub(crate) fn is_boundary(line: &str, start: usize) -> bool {
```

In `crates/adoc-parser/src/parser.rs`, delete the local `const COMMENT_DELIMITER` declaration and add it to the existing import:

```rust
use crate::line_parser::{
    content, find_anchors, find_images, find_includes, find_references, is_verbatim_delimiter,
    parse_attribute, parse_heading, COMMENT_DELIMITER,
};
```

- [ ] **Step 4: Write the implementation**

Prepend to `crates/adoc-parser/src/completion.rs`, above the test module:

```rust
use adoc_core::SourceRange;

use crate::line_parser::{content, is_boundary, is_verbatim_delimiter, COMMENT_DELIMITER};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompletionKind {
    XrefTarget,
    /// An anchor inside `target`. An empty target means the current file.
    XrefAnchor {
        target: String,
    },
    IncludeTarget,
    ImageTarget,
    LocalAnchor,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletionContext {
    pub kind: CompletionKind,
    /// What the author has typed between the start of the target and the cursor.
    pub prefix: String,
    /// The span an accepted completion replaces.
    pub range: SourceRange,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Marker {
    Xref,
    Include,
    Image,
    Anchor,
}

/// What the cursor sits inside, or `None` when it sits inside nothing completable.
///
/// This deliberately does not reuse `Document::references`: the parser records only
/// complete macros, and completion happens precisely while one is half-typed.
#[must_use]
pub fn completion_context(text: &str, offset: usize) -> Option<CompletionContext> {
    if offset > text.len() || !text.is_char_boundary(offset) {
        return None;
    }

    let line_start = text[..offset].rfind('\n').map_or(0, |newline| newline + 1);
    let typed = &text[line_start..offset];

    match block_state(text, line_start) {
        // Asciidoctor expands nothing inside a comment block.
        BlockState::Comment => None,
        // Asciidoctor processes includes inside listing, literal and passthrough blocks,
        // matching `parse_document`. Everything else in the block is content.
        BlockState::Verbatim => line_context(typed, line_start, offset, true),
        BlockState::Body => line_context(typed, line_start, offset, false),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BlockState {
    Body,
    Verbatim,
    Comment,
}

fn block_state(text: &str, line_start: usize) -> BlockState {
    let mut active: Option<&str> = None;
    for line in text[..line_start].split_inclusive('\n') {
        let trimmed = content(line).trim();
        match active {
            Some(delimiter) if trimmed == delimiter => active = None,
            Some(_) => {}
            None if is_verbatim_delimiter(trimmed) => active = Some(trimmed),
            None => {}
        }
    }
    match active {
        None => BlockState::Body,
        Some(COMMENT_DELIMITER) => BlockState::Comment,
        Some(_) => BlockState::Verbatim,
    }
}

fn line_context(
    typed: &str,
    line_start: usize,
    offset: usize,
    includes_only: bool,
) -> Option<CompletionContext> {
    let mut best: Option<(usize, usize, Marker)> = None;

    for (prefix, marker) in [
        ("include::", Marker::Include),
        ("xref:", Marker::Xref),
        ("image:", Marker::Image),
    ] {
        if includes_only && marker != Marker::Include {
            continue;
        }
        let Some(start) = last_macro_start(typed, prefix) else {
            continue;
        };
        let mut target_start = start + prefix.len();
        // `image::` is the block form of the same macro.
        if marker == Marker::Image && typed[target_start..].starts_with(':') {
            target_start += 1;
        }
        if best.is_none_or(|(existing, _, _)| start > existing) {
            best = Some((start, target_start, marker));
        }
    }

    if !includes_only {
        if let Some(start) = typed.rfind("<<") {
            if best.is_none_or(|(existing, _, _)| start > existing) {
                best = Some((start, start + 2, Marker::Anchor));
            }
        }
    }

    let (_, target_start, marker) = best?;
    let target = &typed[target_start..];
    // A `[` closes the target, and no target spans whitespace: the cursor has left it.
    if target.contains('[') || target.chars().any(char::is_whitespace) {
        return None;
    }
    let range_start = line_start + target_start;

    let kind = match marker {
        Marker::Anchor => {
            if target.contains(">>") || target.contains(',') {
                return None;
            }
            CompletionKind::LocalAnchor
        }
        Marker::Include => CompletionKind::IncludeTarget,
        Marker::Image => CompletionKind::ImageTarget,
        Marker::Xref => {
            if let Some((file, anchor)) = target.split_once('#') {
                return Some(CompletionContext {
                    kind: CompletionKind::XrefAnchor {
                        target: file.to_owned(),
                    },
                    prefix: anchor.to_owned(),
                    range: SourceRange::new(range_start + file.len() + 1, offset),
                });
            }
            CompletionKind::XrefTarget
        }
    };

    Some(CompletionContext {
        kind,
        prefix: target.to_owned(),
        range: SourceRange::new(range_start, offset),
    })
}

/// The last occurrence of `prefix` that starts a macro rather than sitting inside a word
/// or behind an escaping backslash.
fn last_macro_start(typed: &str, prefix: &str) -> Option<usize> {
    let mut best = None;
    let mut base = 0;
    while let Some(found) = typed[base..].find(prefix) {
        let start = base + found;
        if is_boundary(typed, start) {
            best = Some(start);
        }
        base = start + prefix.len();
    }
    best
}
```

Add the module to `crates/adoc-parser/src/lib.rs`:

```rust
mod completion;
mod line_parser;
mod parser;

pub use completion::{completion_context, CompletionContext, CompletionKind};
pub use parser::{parse, AsciiDocParser, DocumentParser, ParseResult};
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p adoc-parser completion`
Expected: PASS, 6 tests.

- [ ] **Step 6: Write the failing tests for the cases that make detection trustworthy**

Append inside the same `mod tests`:

```rust
    #[test]
    fn offers_nothing_inside_a_comment_block() {
        let text = "= Demo\n\n////\nSee xref:get";

        assert_eq!(completion_context(text, text.len()), None);
    }

    #[test]
    fn offers_only_includes_inside_a_verbatim_block() {
        let source_block = "= Demo\n\n----\ninclude::example$q";
        let context = completion_context(source_block, source_block.len()).expect("a context");
        assert_eq!(context.kind, CompletionKind::IncludeTarget);

        let xref_in_block = "= Demo\n\n----\nSee xref:get";
        assert_eq!(completion_context(xref_in_block, xref_in_block.len()), None);
    }

    #[test]
    fn resumes_after_a_verbatim_block_closes() {
        let text = "= Demo\n\n----\ncode\n----\n\nSee xref:get";
        let context = completion_context(text, text.len()).expect("a context");

        assert_eq!(context.kind, CompletionKind::XrefTarget);
    }

    #[test]
    fn ignores_an_escaped_macro() {
        let text = "\\xref:get";

        assert_eq!(completion_context(text, text.len()), None);
    }

    #[test]
    fn ignores_a_prefix_inside_a_word() {
        let text = "myxref:get";

        assert_eq!(completion_context(text, text.len()), None);
    }

    #[test]
    fn stops_at_the_closing_bracket() {
        let text = "See xref:page.adoc[Page] and ";

        assert_eq!(completion_context(text, text.len()), None);
    }

    #[test]
    fn stops_at_whitespace_after_the_target() {
        let text = "See xref:page.adoc then";

        assert_eq!(completion_context(text, text.len()), None);
    }

    #[test]
    fn takes_the_nearest_construct_when_a_line_holds_several() {
        let text = "See xref:one.adoc[One] then include::partial$no";
        let context = completion_context(text, text.len()).expect("a context");

        assert_eq!(context.kind, CompletionKind::IncludeTarget);
        assert_eq!(context.prefix, "partial$no");
    }

    #[test]
    fn returns_nothing_for_an_offset_off_the_end() {
        let text = "See xref:get";

        assert_eq!(completion_context(text, text.len() + 5), None);
    }

    #[test]
    fn returns_nothing_on_an_attribute_line() {
        let text = ":toc:";

        assert_eq!(completion_context(text, text.len()), None);
    }
```

- [ ] **Step 7: Run the tests**

Run: `cargo test -p adoc-parser completion`
Expected: PASS, 16 tests. `stops_at_the_closing_bracket` and `stops_at_whitespace_after_the_target` pass because of the `contains('[')` and whitespace guards; `ignores_an_escaped_macro` and `ignores_a_prefix_inside_a_word` pass through `is_boundary`.

- [ ] **Step 8: Run the whole suite to confirm the `COMMENT_DELIMITER` move broke nothing**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check`
Expected: PASS throughout.

- [ ] **Step 9: Commit**

```bash
git add crates/adoc-parser/src/completion.rs crates/adoc-parser/src/lib.rs \
        crates/adoc-parser/src/line_parser.rs crates/adoc-parser/src/parser.rs
git commit -m "feat(parser): detect what a cursor sits inside for completion"
```

---

### Task 2: Range enumerators in `adoc-antora`

**Files:**
- Modify: `crates/adoc-antora/src/catalog.rs`
- Test: `crates/adoc-antora/src/catalog.rs` (existing `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: existing `AntoraCatalog` fields `resources: BTreeMap<AntoraCoordinate, AntoraResource>` and `modules: BTreeMap<ModuleKey, Module>`.
- Produces:
  ```rust
  pub fn resources_in(&self, component: &str, version: Option<&str>,
                      module: &str, family: ResourceFamily) -> impl Iterator<Item = &AntoraResource>
  pub fn modules_of(&self, component: &str, version: Option<&str>) -> impl Iterator<Item = &Module>
  ```

**Background:** These are correct because `AntoraCoordinate` derives `Ord` over its fields in declaration order — *component → version → module → family → relative_path* — so every resource in one module-family is contiguous in the map. `PathBuf::new()` has no components and so sorts below any real path, making it the right lower bound. Do **not** implement these by iterating `resources()` and filtering: that is the whole-workspace scan `CLAUDE.md` forbids.

- [ ] **Step 1: Write the failing tests**

Append inside `crates/adoc-antora/src/catalog.rs`'s `mod tests`:

```rust
    fn two_module_catalog() -> AntoraCatalog {
        let mut catalog = AntoraCatalog::new();
        catalog.insert_component(ComponentDescriptor {
            root: PathBuf::from("."),
            name: "demo".to_owned(),
            title: None,
            version: Some("latest".to_owned()),
            display_version: None,
            start_page: None,
            nav: Vec::new(),
            asciidoc_attributes: BTreeMap::new(),
        });
        for module in ["ROOT", "security"] {
            catalog.insert_module(Module {
                component: "demo".to_owned(),
                version: Some("latest".to_owned()),
                name: module.to_owned(),
                root: PathBuf::from(format!("modules/{module}")),
                nav: None,
            });
            for (family, file) in [
                (ResourceFamily::Page, "index.adoc"),
                (ResourceFamily::Partial, "note.adoc"),
            ] {
                catalog.insert(AntoraResource {
                    coordinate: AntoraCoordinate {
                        component: "demo".to_owned(),
                        version: Some("latest".to_owned()),
                        module: module.to_owned(),
                        family,
                        relative_path: PathBuf::from(file),
                    },
                    source_path: PathBuf::from(format!(
                        "modules/{module}/{}/{file}",
                        family.directory()
                    )),
                });
            }
        }
        catalog
    }

    #[test]
    fn enumerates_only_the_requested_module_and_family() {
        let catalog = two_module_catalog();

        let partials: Vec<_> = catalog
            .resources_in("demo", Some("latest"), "ROOT", ResourceFamily::Partial)
            .map(|resource| resource.source_path.clone())
            .collect();

        assert_eq!(
            partials,
            vec![PathBuf::from("modules/ROOT/partials/note.adoc")],
            "a neighbouring module or family must not leak in"
        );
    }

    #[test]
    fn enumerates_nothing_for_an_unknown_module() {
        let catalog = two_module_catalog();

        assert_eq!(
            catalog
                .resources_in("demo", Some("latest"), "absent", ResourceFamily::Page)
                .count(),
            0
        );
    }

    #[test]
    fn enumerates_the_modules_of_a_component() {
        let catalog = two_module_catalog();

        let names: Vec<_> = catalog
            .modules_of("demo", Some("latest"))
            .map(|module| module.name.clone())
            .collect();

        assert_eq!(names, vec!["ROOT".to_owned(), "security".to_owned()]);
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p adoc-antora enumerates`
Expected: compile failure — no method named `resources_in` / `modules_of`.

- [ ] **Step 3: Write the implementation**

Add to `impl AntoraCatalog` in `crates/adoc-antora/src/catalog.rs`, beside `resources()`:

```rust
    /// Every resource in one module's family, in path order.
    ///
    /// This is a range over the ordered map rather than a filter over every resource:
    /// `AntoraCoordinate` orders component, version, module, family, then path, so the
    /// matching entries are contiguous and the cost is proportional to the answer.
    pub fn resources_in(
        &self,
        component: &str,
        version: Option<&str>,
        module: &str,
        family: ResourceFamily,
    ) -> impl Iterator<Item = &AntoraResource> {
        let start = AntoraCoordinate {
            component: component.to_owned(),
            version: version.map(str::to_owned),
            module: module.to_owned(),
            family,
            relative_path: PathBuf::new(),
        };
        let component = component.to_owned();
        let version = version.map(str::to_owned);
        let module = module.to_owned();
        self.resources
            .range(start..)
            .take_while(move |(coordinate, _)| {
                coordinate.component == component
                    && coordinate.version == version
                    && coordinate.module == module
                    && coordinate.family == family
            })
            .map(|(_, resource)| resource)
    }

    /// Every module of one component version, in name order.
    pub fn modules_of(
        &self,
        component: &str,
        version: Option<&str>,
    ) -> impl Iterator<Item = &Module> {
        let start = ModuleKey {
            component: component.to_owned(),
            version: version.map(str::to_owned),
            name: String::new(),
        };
        let component = component.to_owned();
        let version = version.map(str::to_owned);
        self.modules
            .range(start..)
            .take_while(move |(key, _)| key.component == component && key.version == version)
            .map(|(_, module)| module)
    }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p adoc-antora`
Expected: PASS, existing tests plus the 3 new ones.

- [ ] **Step 5: Commit**

```bash
git add crates/adoc-antora/src/catalog.rs
git commit -m "feat(antora): enumerate a module family without scanning the catalog"
```

---

### Task 3: Range enumerators and directory listing in `adoc-index`

**Files:**
- Modify: `crates/adoc-index/src/index.rs`
- Modify: `crates/adoc-index/src/workspace.rs`
- Modify: `crates/adoc-index/src/lib.rs`
- Test: both modified source files (`#[cfg(test)] mod tests`)

**Interfaces:**
- Produces:
  ```rust
  impl WorkspaceIndex {
      pub fn anchors_in(&self, path: &Path) -> impl Iterator<Item = (&str, &AnchorLocation)>
      pub fn files_under(&self, directory: &Path) -> impl Iterator<Item = &FileEntry>
  }
  pub struct DirectoryEntry { pub name: String, pub is_directory: bool }
  pub fn list_directory(directory: &Path) -> Vec<DirectoryEntry>
  ```

**Background:** `AnchorKey` orders *path → id*, and `PathBuf` compares component-wise, so both enumerators are contiguous ranges. `anchors_in` yields each id once, taking the first location, which matches `resolve_anchor` returning `locations.first()`. `list_directory` is the *only* filesystem read on the completion path: the index holds `.adoc` files only, but `include::../code/query.sql[]` is ordinary AsciiDoc.

- [ ] **Step 1: Write the failing tests for the index enumerators**

Append inside `crates/adoc-index/src/index.rs`'s `mod tests`:

```rust
    #[test]
    fn enumerates_the_anchors_of_one_file_only() {
        let mut index = WorkspaceIndex::new();
        index.index_source(
            PathBuf::from("docs/guide.adoc"),
            "[[intro]]\n== Intro\n\n[[detail]]\n== Detail\n",
        );
        index.index_source(PathBuf::from("docs/other.adoc"), "[[elsewhere]]\n== Away\n");

        let mut ids: Vec<_> = index
            .anchors_in(Path::new("docs/guide.adoc"))
            .map(|(id, _)| id.to_owned())
            .collect();
        ids.sort();

        assert!(ids.contains(&"intro".to_owned()));
        assert!(ids.contains(&"detail".to_owned()));
        assert!(
            !ids.contains(&"elsewhere".to_owned()),
            "another file's anchors must not leak in"
        );
    }

    #[test]
    fn enumerates_the_files_beneath_a_directory() {
        let mut index = WorkspaceIndex::new();
        index.index_source(PathBuf::from("docs/guides/one.adoc"), "= One\n");
        index.index_source(PathBuf::from("docs/guides/two.adoc"), "= Two\n");
        index.index_source(PathBuf::from("docs/reference/three.adoc"), "= Three\n");

        let paths: Vec<_> = index
            .files_under(Path::new("docs/guides"))
            .map(|entry| entry.path.clone())
            .collect();

        assert_eq!(
            paths,
            vec![
                PathBuf::from("docs/guides/one.adoc"),
                PathBuf::from("docs/guides/two.adoc"),
            ]
        );
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p adoc-index enumerates`
Expected: compile failure — no method named `anchors_in` / `files_under`.

- [ ] **Step 3: Write the index enumerators**

Add to `impl WorkspaceIndex` in `crates/adoc-index/src/index.rs`, beside `resolve_anchor`:

```rust
    /// Every anchor id declared in one file, in id order, with its first location.
    ///
    /// A range over the ordered map, not a scan: `AnchorKey` orders path then id.
    pub fn anchors_in(&self, path: &Path) -> impl Iterator<Item = (&str, &AnchorLocation)> {
        let path = normalize_path(path);
        let start = AnchorKey {
            path: path.clone(),
            id: String::new(),
        };
        self.anchors
            .range(start..)
            .take_while(move |(key, _)| key.path == path)
            .filter_map(|(key, locations)| {
                locations
                    .first()
                    .map(|location| (key.id.as_str(), location))
            })
    }

    /// Every indexed file beneath a directory, in path order.
    pub fn files_under(&self, directory: &Path) -> impl Iterator<Item = &FileEntry> {
        let directory = normalize_path(directory);
        self.files
            .range(directory.clone()..)
            .take_while(move |(path, _)| path.starts_with(&directory))
            .map(|(_, entry)| entry)
    }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p adoc-index enumerates`
Expected: PASS, 2 tests.

- [ ] **Step 5: Write the failing test for `list_directory`**

Append inside `crates/adoc-index/src/workspace.rs`'s `mod tests`:

```rust
    #[test]
    fn lists_a_directory_for_completion() {
        use super::list_directory;

        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/antora-single-component/modules/ROOT");

        let names: Vec<_> = list_directory(&root)
            .into_iter()
            .map(|entry| entry.name)
            .collect();

        assert_eq!(
            names,
            vec![
                "attachments".to_owned(),
                "examples".to_owned(),
                "images".to_owned(),
                "nav.adoc".to_owned(),
                "pages".to_owned(),
                "partials".to_owned(),
            ]
        );
        assert!(list_directory(&root.join("absent")).is_empty());
    }

    #[test]
    fn lists_non_asciidoc_files_the_index_does_not_hold() {
        use super::list_directory;

        let examples = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/antora-single-component/modules/ROOT/examples");

        let names: Vec<_> = list_directory(&examples)
            .into_iter()
            .map(|entry| entry.name)
            .collect();

        assert_eq!(names, vec!["sample.json".to_owned()]);
    }
```

- [ ] **Step 6: Run the tests to verify they fail**

Run: `cargo test -p adoc-index lists_`
Expected: compile failure — `list_directory` is not defined.

- [ ] **Step 7: Write `list_directory`**

Add to `crates/adoc-index/src/workspace.rs`, after `is_asciidoc_path`:

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectoryEntry {
    pub name: String,
    pub is_directory: bool,
}

/// One shallow directory listing for path completion, sorted by name.
///
/// Completion needs targets the index does not hold — `query.sql`, `diagram.png` — so this
/// is the one filesystem read on the completion path. An unreadable directory lists
/// nothing rather than failing: completion never reports an error.
#[must_use]
pub fn list_directory(directory: &Path) -> Vec<DirectoryEntry> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };

    let mut listed = Vec::new();
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let is_directory = file_type.is_dir();
        if is_directory && DEFAULT_IGNORED_DIRECTORIES.contains(&name.as_str()) {
            continue;
        }
        listed.push(DirectoryEntry { name, is_directory });
    }
    listed.sort_by(|left, right| left.name.cmp(&right.name));
    listed
}
```

Re-export from `crates/adoc-index/src/lib.rs`:

```rust
pub use workspace::{
    is_asciidoc_path, list_directory, normalize_path, DirectoryEntry, DEFAULT_IGNORED_DIRECTORIES,
};
```

- [ ] **Step 8: Run the tests to verify they pass**

Run: `cargo test -p adoc-index`
Expected: PASS, existing tests plus the 4 new ones.

- [ ] **Step 9: Commit**

```bash
git add crates/adoc-index/src/index.rs crates/adoc-index/src/workspace.rs crates/adoc-index/src/lib.rs
git commit -m "feat(index): enumerate anchors, files and directory entries for completion"
```

---

### Task 4: Handler skeleton and anchor candidates

**Files:**
- Create: `crates/adoc-ls/src/handlers/completion.rs`
- Modify: `crates/adoc-ls/src/handlers/mod.rs`
- Modify: `crates/adoc-ls/src/handlers/definition.rs`
- Test: `crates/adoc-ls/src/handlers/completion.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `adoc_parser::{completion_context, CompletionContext, CompletionKind}` (Task 1); `WorkspaceIndex::anchors_in` (Task 3).
- Produces:
  ```rust
  pub struct Candidate {
      pub label: String,
      pub detail: Option<String>,
      pub sort_text: String,
      pub kind: CandidateKind,
      pub range: SourceRange,
  }
  pub enum CandidateKind { Page, Resource, Module, Family, Directory, Anchor }
  pub fn completion_at_offset(index: &WorkspaceIndex, antora: &AntoraCatalog,
                              current_path: &Path, document: &Document,
                              offset: usize) -> Vec<Candidate>
  // in definition.rs:
  pub fn reference_target_path(index: &WorkspaceIndex, antora: &AntoraCatalog,
                               current_path: &Path, target: &str) -> Option<PathBuf>
  ```

**Background:** `label` is what gets inserted; the `TextEdit` in Task 5 replaces `range` with it. Anchor contexts come first because they exercise the whole shape — context in, filtered candidates out — with the least machinery. `reference_target_path` lives in `definition.rs` so that "which file does this target name" has exactly one answer in the codebase.

- [ ] **Step 1: Write the failing test for local anchors**

Create `crates/adoc-ls/src/handlers/completion.rs` with only the test module:

```rust
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

        let labels: Vec<_> = completion_at_offset(
            &index,
            &AntoraCatalog::new(),
            path,
            &document,
            text.len(),
        )
        .into_iter()
        .map(|candidate| candidate.label)
        .collect();

        assert!(labels.contains(&"intro".to_owned()));
        assert!(labels.contains(&"detail".to_owned()));
    }

    #[test]
    fn filters_anchors_by_what_has_been_typed() {
        let path = Path::new("/docs/guide.adoc");
        let text = "[[intro]]\n== Intro\n\n[[detail]]\n== Detail\n\nSee <<det";
        let mut index = WorkspaceIndex::new();
        index.index_source(path, text);
        let document = parse("file:///docs/guide.adoc", text).document;

        let labels: Vec<_> = completion_at_offset(
            &index,
            &AntoraCatalog::new(),
            path,
            &document,
            text.len(),
        )
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

        assert!(completion_at_offset(
            &index,
            &AntoraCatalog::new(),
            path,
            &document,
            text.len()
        )
        .is_empty());
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p adoc-ls completes_anchors`
Expected: compile failure — `completion_at_offset` is not defined, module not declared.

- [ ] **Step 3: Add `reference_target_path` to `definition.rs`**

Append to `crates/adoc-ls/src/handlers/definition.rs`, after `definition_at_offset`:

```rust
/// The file a reference target names, resolved in the same order `definition_at_offset`
/// uses: Antora resource ID first when the file sits in a module, then a relative path.
///
/// An empty target means the current file, which is how `xref:#anchor` is written.
#[must_use]
pub fn reference_target_path(
    index: &WorkspaceIndex,
    antora: &AntoraCatalog,
    current_path: &Path,
    target: &str,
) -> Option<PathBuf> {
    if target.is_empty() {
        return Some(current_path.to_path_buf());
    }

    if let Some(context) = antora.context_for_path(current_path) {
        if let Ok(id) = parse_resource_id(target) {
            if let Ok(resource) = AntoraResolver::resolve(antora, &id, &context) {
                return Some(resource.source_path.clone());
            }
        }
    }

    let relative = current_path.parent()?.join(target);
    let relative = adoc_index::normalize_path(&relative);
    (index.file(&relative).is_some() || relative.is_file()).then_some(relative)
}
```

- [ ] **Step 4: Write the handler skeleton and anchor assembly**

Prepend to `crates/adoc-ls/src/handlers/completion.rs`, above the test module:

```rust
use std::path::Path;

use adoc_antora::AntoraCatalog;
use adoc_core::{Document, SourceRange};
use adoc_index::WorkspaceIndex;
use adoc_parser::{completion_context, CompletionKind};

use crate::handlers::definition::reference_target_path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateKind {
    Page,
    Resource,
    Module,
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

fn anchor_candidates(index: &WorkspaceIndex, path: &Path, range: SourceRange) -> Vec<Candidate> {
    index
        .anchors_in(path)
        .map(|(id, _)| Candidate {
            label: id.to_owned(),
            detail: None,
            sort_text: format!("0{id}"),
            kind: CandidateKind::Anchor,
            range,
        })
        .collect()
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
```

Register the module in `crates/adoc-ls/src/handlers/mod.rs`, keeping the list alphabetical:

```rust
pub mod code_actions;
pub mod completion;
pub mod definition;
pub mod diagnostics;
pub mod document_symbols;
pub mod execute_command;
pub mod includes;
pub mod render_source;
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p adoc-ls completion`
Expected: PASS, 4 tests.

- [ ] **Step 6: Commit**

```bash
git add crates/adoc-ls/src/handlers/completion.rs crates/adoc-ls/src/handlers/mod.rs \
        crates/adoc-ls/src/handlers/definition.rs
git commit -m "feat(ls): complete anchors in the current and referenced files"
```

---

### Task 5: LSP surface — capability, dispatch, round-trip

**Files:**
- Modify: `crates/adoc-ls/src/capabilities.rs`
- Modify: `crates/adoc-ls/src/protocol.rs:113-131` (dispatch) and near `definition_response`
- Modify: `crates/adoc-ls/tests/stdio.rs`
- Test: `crates/adoc-ls/src/capabilities.rs`, `crates/adoc-ls/tests/stdio.rs`

**Interfaces:**
- Consumes: `handlers::completion::{completion_at_offset, Candidate, CandidateKind}` (Task 4); `PositionEncoding::{offset, range}`.
- Produces: a working `textDocument/completion` endpoint. Later tasks only add candidate sources; this wiring does not change again.

**Background:** After this task the feature is live end-to-end with anchors only, which is the point — everything after it is additive. Trigger characters fire indiscriminately (`:` also fires on `:toc:`); that is intended, because `completion_context` returns `None` there and the reply is an empty list. Detection is the guard, not the trigger set.

- [ ] **Step 1: Write the failing capability test**

Append inside `crates/adoc-ls/src/capabilities.rs`'s `mod tests`:

```rust
    #[test]
    fn advertises_completion_with_its_trigger_characters() {
        let capabilities = server_capabilities(PositionEncoding::Utf16);
        let completion = capabilities
            .completion_provider
            .expect("completion provider");

        let triggers = completion.trigger_characters.expect("trigger characters");
        for expected in [":", "$", "#", "/", "<"] {
            assert!(
                triggers.iter().any(|trigger| trigger == expected),
                "`{expected}` must trigger completion: {triggers:?}"
            );
        }
        assert_eq!(completion.resolve_provider, Some(false));
    }
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p adoc-ls advertises_completion`
Expected: FAIL — `completion_provider` is `None`, panicking on `expect("completion provider")`.

- [ ] **Step 3: Advertise the capability**

In `crates/adoc-ls/src/capabilities.rs`, add `CompletionOptions` to the `lsp_types` import and the field to `ServerCapabilities`, after `definition_provider`:

```rust
        // The trigger set is only a hint about when to ask. `completion_context` decides
        // whether there is anything to offer, so `:` firing on an attribute line is fine.
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec![
                ":".to_owned(),
                "$".to_owned(),
                "#".to_owned(),
                "/".to_owned(),
                "<".to_owned(),
            ]),
            resolve_provider: Some(false),
            ..CompletionOptions::default()
        }),
```

- [ ] **Step 4: Run it to verify it passes**

Run: `cargo test -p adoc-ls advertises_completion`
Expected: PASS.

- [ ] **Step 5: Write the failing stdio round-trip test**

Append to `crates/adoc-ls/tests/stdio.rs`, extending the `lsp_types` import with `CompletionResponse`, `Position`, `TextDocumentIdentifier`, `Url`, and `request::Completion`:

```rust
#[test]
fn binary_answers_a_completion_request() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_adoc-ls"))
        .arg("--stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("start adoc-ls");
    let mut stdin = child.stdin.take().expect("child stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("child stdout"));

    Message::Request(Request::new(
        RequestId::from(1),
        Initialize::METHOD.to_owned(),
        InitializeParams::default(),
    ))
    .write(&mut stdin)
    .expect("send initialize");
    assert_success_response(read_message(&mut stdout), RequestId::from(1));
    Message::Notification(Notification::new(
        Initialized::METHOD.to_owned(),
        InitializedParams {},
    ))
    .write(&mut stdin)
    .expect("send initialized");

    let uri = "file:///docs/guide.adoc";
    let text = "[[intro]]\n== Intro\n\nSee <<";
    Message::Notification(Notification::new(
        "textDocument/didOpen".to_owned(),
        serde_json::json!({
            "textDocument": { "uri": uri, "languageId": "asciidoc", "version": 1, "text": text },
        }),
    ))
    .write(&mut stdin)
    .expect("send didOpen");
    // didOpen publishes diagnostics before anything else arrives.
    let _ = read_message(&mut stdout);

    Message::Request(Request::new(
        RequestId::from(2),
        "textDocument/completion".to_owned(),
        serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": 3, "character": 6 },
        }),
    ))
    .write(&mut stdin)
    .expect("send completion");

    let Message::Response(response) = read_message(&mut stdout) else {
        panic!("expected a completion response");
    };
    let result: Option<CompletionResponse> =
        serde_json::from_value(response.response_result.expect("completion succeeded"))
            .expect("decode completion");
    let Some(CompletionResponse::List(list)) = result else {
        panic!("expected a completion list");
    };
    assert!(list.is_incomplete);
    assert!(
        list.items.iter().any(|item| item.label == "intro"),
        "the anchor declared in the buffer must be offered: {:?}",
        list.items
    );

    Message::Request(Request::new(
        RequestId::from(3),
        Shutdown::METHOD.to_owned(),
        (),
    ))
    .write(&mut stdin)
    .expect("send shutdown");
    let _ = read_message(&mut stdout);
    Message::Notification(Notification::new(Exit::METHOD.to_owned(), ()))
        .write(&mut stdin)
        .expect("send exit");
    assert!(child.wait().expect("wait for adoc-ls").success());
}
```

- [ ] **Step 6: Run it to verify it fails**

Run: `cargo test -p adoc-ls --test stdio binary_answers_a_completion_request`
Expected: FAIL — the server replies `unsupported request \`textDocument/completion\``, so `response_result` is `Err` and `.expect("completion succeeded")` panics.

- [ ] **Step 7: Wire the dispatch arm**

In `crates/adoc-ls/src/protocol.rs`, add `Completion` to the `lsp_types::request` import, add `CompletionItem`, `CompletionItemKind`, `CompletionList`, `CompletionParams`, `CompletionResponse`, `CompletionTextEdit`, and `TextEdit` to the `lsp_types` import, and add `completion_at_offset`, `Candidate`, `CandidateKind` to the handlers import. Then add the arm beside `GotoDefinition`:

```rust
            Completion::METHOD => self.request_response::<CompletionParams, _>(request, |params| {
                self.completion_response(params)
            }),
```

And the response builder, beside `definition_response`:

```rust
    fn completion_response(&self, params: CompletionParams) -> Option<CompletionResponse> {
        let document_uri = params.text_document_position.text_document.uri.as_str();
        let document = &self.state.documents.get(document_uri)?.document;
        let offset = self
            .encoding
            .offset(&document.text, params.text_document_position.position)?;
        let current_path = document_path(document_uri);
        let candidates = completion_at_offset(
            &self.state.index,
            &self.state.antora,
            &current_path,
            document,
            offset,
        );

        let items = candidates
            .into_iter()
            .filter_map(|candidate| self.completion_item(&document.text, candidate))
            .collect();
        Some(CompletionResponse::List(CompletionList {
            // The candidate set narrows as the target grows, so the client must ask again.
            is_incomplete: true,
            items,
        }))
    }

    fn completion_item(&self, text: &str, candidate: Candidate) -> Option<CompletionItem> {
        let range = self.encoding.range(text, candidate.range)?;
        Some(CompletionItem {
            label: candidate.label.clone(),
            kind: Some(match candidate.kind {
                CandidateKind::Page | CandidateKind::Resource => CompletionItemKind::FILE,
                CandidateKind::Module => CompletionItemKind::MODULE,
                CandidateKind::Family => CompletionItemKind::KEYWORD,
                CandidateKind::Directory => CompletionItemKind::FOLDER,
                CandidateKind::Anchor => CompletionItemKind::REFERENCE,
            }),
            detail: candidate.detail,
            sort_text: Some(candidate.sort_text),
            filter_text: Some(candidate.label.clone()),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                range,
                new_text: candidate.label,
            })),
            ..CompletionItem::default()
        })
    }
```

- [ ] **Step 8: Run the round-trip to verify it passes**

Run: `cargo test -p adoc-ls --test stdio binary_answers_a_completion_request`
Expected: PASS.

- [ ] **Step 9: Run the full baseline**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: PASS throughout.

- [ ] **Step 10: Commit**

```bash
git add crates/adoc-ls/src/capabilities.rs crates/adoc-ls/src/protocol.rs crates/adoc-ls/tests/stdio.rs
git commit -m "feat(ls): answer textDocument/completion over stdio"
```

---

### Task 6: Antora page, family and resource candidates

**Files:**
- Modify: `crates/adoc-ls/src/handlers/completion.rs`
- Test: `crates/adoc-ls/src/handlers/completion.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `AntoraCatalog::{context_for_path, resources_in, modules_of}` (Task 2); `WorkspaceIndex::file`; `completion_at_offset` (Task 4).
- Produces: no new public names. `completion_at_offset` gains behaviour for `XrefTarget`, `IncludeTarget` and `ImageTarget` inside an Antora module.

**Background:** `sortText` carries the ordering, not list position: `0` for the current module's bare IDs and `1` for module-qualified ones, so the current module ranks first while the client stays free to filter. `detail` is the target's document title, read from the index, which is why the server advertises `resolve_provider: false`. The fixture `tests/fixtures/antora-single-component` has two modules (`ROOT`, `security`) and one resource per family, which is enough for every assertion here.

- [ ] **Step 1: Write the failing tests**

Append inside `crates/adoc-ls/src/handlers/completion.rs`'s `mod tests`:

```rust
    use adoc_antora::discover_antora_workspace;

    fn antora_fixture() -> (WorkspaceIndex, AntoraCatalog, std::path::PathBuf) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/antora-single-component");
        let mut index = WorkspaceIndex::new();
        index.index_roots(&[root.clone()]).expect("index fixture");
        let catalog = discover_antora_workspace(&[root.clone()])
            .expect("discover fixture")
            .catalog;
        (index, catalog, root)
    }

    #[test]
    fn ranks_the_current_module_above_other_modules() {
        let (index, catalog, root) = antora_fixture();
        let path = root.join("modules/ROOT/pages/index.adoc");
        let text = "= Index\n\nSee xref:";
        let document = parse("file:///index.adoc", text).document;

        let candidates = completion_at_offset(&index, &catalog, &path, &document, text.len());
        let labels: Vec<_> = candidates
            .iter()
            .map(|candidate| candidate.label.clone())
            .collect();

        assert!(
            labels.contains(&"index.adoc".to_owned()),
            "the current module's pages are offered as bare ids: {labels:?}"
        );
        assert!(
            labels.contains(&"security:authentication.adoc".to_owned()),
            "other modules are offered module-qualified: {labels:?}"
        );
        let bare = candidates
            .iter()
            .find(|candidate| candidate.label == "index.adoc")
            .expect("bare id");
        let qualified = candidates
            .iter()
            .find(|candidate| candidate.label == "security:authentication.adoc")
            .expect("qualified id");
        assert!(
            bare.sort_text < qualified.sort_text,
            "the current module must sort first: {} vs {}",
            bare.sort_text,
            qualified.sort_text
        );
    }

    #[test]
    fn offers_the_family_prefixes_before_a_dollar_is_typed() {
        let (index, catalog, root) = antora_fixture();
        let path = root.join("modules/ROOT/pages/index.adoc");
        let text = "= Index\n\ninclude::";
        let document = parse("file:///index.adoc", text).document;

        let labels: Vec<_> = completion_at_offset(&index, &catalog, &path, &document, text.len())
            .into_iter()
            .map(|candidate| candidate.label)
            .collect();

        for family in [
            "page$",
            "partial$",
            "example$",
            "image$",
            "attachment$",
        ] {
            assert!(
                labels.contains(&family.to_owned()),
                "`{family}` must be offered: {labels:?}"
            );
        }
    }

    #[test]
    fn offers_a_family_s_resources_once_the_dollar_is_typed() {
        let (index, catalog, root) = antora_fixture();
        let path = root.join("modules/ROOT/pages/index.adoc");
        let text = "= Index\n\ninclude::partial$";
        let document = parse("file:///index.adoc", text).document;

        let labels: Vec<_> = completion_at_offset(&index, &catalog, &path, &document, text.len())
            .into_iter()
            .map(|candidate| candidate.label)
            .collect();

        assert!(
            labels.contains(&"partial$welcome.adoc".to_owned()),
            "{labels:?}"
        );
        assert!(
            !labels.contains(&"partial$token-note.adoc".to_owned()),
            "another module's partials must not leak in: {labels:?}"
        );
    }

    #[test]
    fn offers_image_resources_for_an_image_macro() {
        let (index, catalog, root) = antora_fixture();
        let path = root.join("modules/ROOT/pages/index.adoc");
        let text = "= Index\n\nimage::";
        let document = parse("file:///index.adoc", text).document;

        let labels: Vec<_> = completion_at_offset(&index, &catalog, &path, &document, text.len())
            .into_iter()
            .map(|candidate| candidate.label)
            .collect();

        assert!(labels.contains(&"architecture.svg".to_owned()), "{labels:?}");
    }

    #[test]
    fn carries_the_target_title_as_detail() {
        let (index, catalog, root) = antora_fixture();
        let path = root.join("modules/ROOT/pages/index.adoc");
        let text = "= Index\n\nSee xref:security:";
        let document = parse("file:///index.adoc", text).document;

        let detail = completion_at_offset(&index, &catalog, &path, &document, text.len())
            .into_iter()
            .find(|candidate| candidate.label == "security:authentication.adoc")
            .and_then(|candidate| candidate.detail);

        assert!(detail.is_some(), "a page candidate carries its title");
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p adoc-ls completion`
Expected: FAIL — the four `XrefTarget`/`IncludeTarget`/`ImageTarget` tests return empty lists, so every `assert!(labels.contains(...))` fails.

- [ ] **Step 3: Write the Antora candidate builders**

In `crates/adoc-ls/src/handlers/completion.rs`, extend the imports and replace the placeholder match arms:

```rust
use adoc_antora::{AntoraCatalog, AntoraContext, ResourceFamily};
```

```rust
        CompletionKind::XrefTarget => antora_page_candidates(index, antora, current_path, context.range)
            .unwrap_or_default(),
        CompletionKind::IncludeTarget => {
            antora_include_candidates(index, antora, current_path, &context.prefix, context.range)
                .unwrap_or_default()
        }
        CompletionKind::ImageTarget => {
            antora_family_candidates(index, antora, current_path, ResourceFamily::Image, "", context.range)
                .unwrap_or_default()
        }
```

Then add the builders:

```rust
fn antora_page_candidates(
    index: &WorkspaceIndex,
    antora: &AntoraCatalog,
    current_path: &Path,
    range: SourceRange,
) -> Option<Vec<Candidate>> {
    let context = antora.context_for_path(current_path)?;
    let mut candidates = Vec::new();

    // The current module's pages are written bare, the way Antora authors write them.
    for resource in antora.resources_in(
        &context.component,
        context.version.as_deref(),
        &context.module,
        ResourceFamily::Page,
    ) {
        let label = resource.coordinate.relative_path.to_string_lossy().into_owned();
        candidates.push(Candidate {
            detail: title_of(index, &resource.source_path),
            sort_text: format!("0{label}"),
            kind: CandidateKind::Page,
            range,
            label,
        });
    }

    // Every other module of the same component, module-qualified and ranked below.
    for module in antora.modules_of(&context.component, context.version.as_deref()) {
        if module.name == context.module {
            continue;
        }
        for resource in antora.resources_in(
            &context.component,
            context.version.as_deref(),
            &module.name,
            ResourceFamily::Page,
        ) {
            let label = format!(
                "{}:{}",
                module.name,
                resource.coordinate.relative_path.to_string_lossy()
            );
            candidates.push(Candidate {
                detail: title_of(index, &resource.source_path),
                sort_text: format!("1{label}"),
                kind: CandidateKind::Page,
                range,
                label,
            });
        }
    }

    Some(candidates)
}

fn antora_include_candidates(
    index: &WorkspaceIndex,
    antora: &AntoraCatalog,
    current_path: &Path,
    prefix: &str,
    range: SourceRange,
) -> Option<Vec<Candidate>> {
    let context = antora.context_for_path(current_path)?;

    // Confirms the file sits in a module; without that there is nothing Antora to offer.
    let _ = context;

    let Some((family_name, _)) = prefix.split_once('$') else {
        // No family chosen yet: offer the families themselves.
        return Some(
            ResourceFamily::ALL
                .iter()
                .map(|family| {
                    let label = format!("{family}$");
                    Candidate {
                        detail: None,
                        sort_text: format!("0{label}"),
                        kind: CandidateKind::Family,
                        range,
                        label,
                    }
                })
                .collect(),
        );
    };

    let family = family_name.parse::<ResourceFamily>().ok()?;
    antora_family_candidates(
        index,
        antora,
        current_path,
        family,
        &format!("{family}$"),
        range,
    )
}

fn antora_family_candidates(
    index: &WorkspaceIndex,
    antora: &AntoraCatalog,
    current_path: &Path,
    family: ResourceFamily,
    label_prefix: &str,
    range: SourceRange,
) -> Option<Vec<Candidate>> {
    let context: AntoraContext = antora.context_for_path(current_path)?;
    Some(
        antora
            .resources_in(
                &context.component,
                context.version.as_deref(),
                &context.module,
                family,
            )
            .map(|resource| {
                let label = format!(
                    "{label_prefix}{}",
                    resource.coordinate.relative_path.to_string_lossy()
                );
                Candidate {
                    detail: title_of(index, &resource.source_path),
                    sort_text: format!("0{label}"),
                    kind: CandidateKind::Resource,
                    range,
                    label,
                }
            })
            .collect(),
    )
}

fn title_of(index: &WorkspaceIndex, path: &Path) -> Option<String> {
    index
        .file(path)?
        .document
        .title
        .as_ref()
        .map(|title| title.text.clone())
}
```

Simplify `antora_include_candidates`'s tail to drop the placeholder `context` juggling once it compiles — the final form is:

```rust
    let family = family_name.parse::<ResourceFamily>().ok()?;
    antora_family_candidates(
        index,
        antora,
        current_path,
        family,
        &format!("{family}$"),
        range,
    )
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p adoc-ls completion`
Expected: PASS, 9 tests. Note `filter_by_prefix` runs after these builders, which is what makes `include::partial$` narrow to labels containing `partial$`, and `xref:security:` narrow to the `security` module.

- [ ] **Step 5: Run clippy, which is strict here**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS. If it flags `too_many_arguments` on `antora_family_candidates`, group `family` and `label_prefix` into a small private struct rather than allowing the lint.

- [ ] **Step 6: Commit**

```bash
git add crates/adoc-ls/src/handlers/completion.rs
git commit -m "feat(ls): complete Antora pages, families and module resources"
```

---

### Task 7: Path candidates outside Antora

**Files:**
- Modify: `crates/adoc-ls/src/handlers/completion.rs`
- Test: `crates/adoc-ls/src/handlers/completion.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `adoc_index::{list_directory, DirectoryEntry}` (Task 3); `WorkspaceIndex::files_under`.
- Produces: no new public names. The three target kinds fall back to path candidates when there is no Antora context.

**Background:** The path completer splits the typed prefix at the last `/`, resolves that directory against the current document, and merges indexed `.adoc` files with one shallow `read_dir`. The `read_dir` is what makes `include::../code/query.sql[]` completable at all, since the index holds only `.adoc` files. Because everything resolves relative to the typed prefix, `../shared/` needs no special case. The label keeps the directory part the author typed, so accepting a candidate leaves a valid relative path.

- [ ] **Step 1: Write the failing tests**

Append inside `crates/adoc-ls/src/handlers/completion.rs`'s `mod tests`:

```rust
    #[test]
    fn completes_relative_paths_outside_antora() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/xrefs");
        let mut index = WorkspaceIndex::new();
        index.index_roots(&[root.clone()]).expect("index fixture");
        let path = root.join("index.adoc");
        let text = "= Index\n\nSee xref:";
        let document = parse("file:///index.adoc", text).document;

        let labels: Vec<_> =
            completion_at_offset(&index, &AntoraCatalog::new(), &path, &document, text.len())
                .into_iter()
                .map(|candidate| candidate.label)
                .collect();

        assert!(labels.contains(&"other.adoc".to_owned()), "{labels:?}");
    }

    #[test]
    fn completes_non_asciidoc_include_targets_from_the_filesystem() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/antora-single-component");
        let mut index = WorkspaceIndex::new();
        index.index_roots(&[root.clone()]).expect("index fixture");
        // No catalog, so this exercises the plain-workspace path.
        let path = root.join("modules/ROOT/pages/index.adoc");
        let text = "= Index\n\ninclude::../examples/";
        let document = parse("file:///index.adoc", text).document;

        let labels: Vec<_> =
            completion_at_offset(&index, &AntoraCatalog::new(), &path, &document, text.len())
                .into_iter()
                .map(|candidate| candidate.label)
                .collect();

        assert!(
            labels.contains(&"../examples/sample.json".to_owned()),
            "a non-AsciiDoc include target must come from the directory read: {labels:?}"
        );
    }

    #[test]
    fn offers_directories_as_path_candidates() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/antora-nested-pages");
        let mut index = WorkspaceIndex::new();
        index.index_roots(&[root.clone()]).expect("index fixture");
        let path = root.join("modules/ROOT/pages/index.adoc");
        let text = "= Index\n\nSee xref:";
        let document = parse("file:///index.adoc", text).document;

        let candidates =
            completion_at_offset(&index, &AntoraCatalog::new(), &path, &document, text.len());
        let directory = candidates
            .iter()
            .find(|candidate| candidate.label == "guides/")
            .expect("a directory candidate");

        assert_eq!(directory.kind, super::CandidateKind::Directory);
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p adoc-ls completes_relative_paths offers_directories completes_non_asciidoc`
Expected: FAIL — all three return empty lists, because without an Antora context the builders from Task 6 return `unwrap_or_default()`.

- [ ] **Step 3: Write the path completer**

In `crates/adoc-ls/src/handlers/completion.rs`, extend the import and replace the three `unwrap_or_default()` tails so each falls back:

```rust
use adoc_index::{list_directory, WorkspaceIndex};
```

```rust
        CompletionKind::XrefTarget => {
            antora_page_candidates(index, antora, current_path, context.range)
                .unwrap_or_else(|| path_candidates(index, current_path, &context.prefix, context.range))
        }
        CompletionKind::IncludeTarget => {
            antora_include_candidates(index, antora, current_path, &context.prefix, context.range)
                .unwrap_or_else(|| path_candidates(index, current_path, &context.prefix, context.range))
        }
        CompletionKind::ImageTarget => antora_family_candidates(
            index,
            antora,
            current_path,
            ResourceFamily::Image,
            "",
            context.range,
        )
        .unwrap_or_else(|| path_candidates(index, current_path, &context.prefix, context.range)),
```

Then add:

```rust
/// Relative-path candidates for a workspace with no Antora catalog.
///
/// The typed prefix is split at its last `/`: the left half names the directory to look
/// in, and is kept on every label so that accepting a candidate leaves a valid path. That
/// is also why `../shared/` needs no special handling.
fn path_candidates(
    index: &WorkspaceIndex,
    current_path: &Path,
    prefix: &str,
    range: SourceRange,
) -> Vec<Candidate> {
    let (typed_directory, _) = prefix.rsplit_once('/').unwrap_or(("", prefix));
    let label_prefix = if typed_directory.is_empty() {
        String::new()
    } else {
        format!("{typed_directory}/")
    };
    let Some(base) = current_path.parent() else {
        return Vec::new();
    };
    let directory = adoc_index::normalize_path(&base.join(typed_directory));

    let mut candidates = Vec::new();
    for entry in list_directory(&directory) {
        let label = format!(
            "{label_prefix}{}{}",
            entry.name,
            if entry.is_directory { "/" } else { "" }
        );
        let detail = (!entry.is_directory)
            .then(|| title_of(index, &directory.join(&entry.name)))
            .flatten();
        candidates.push(Candidate {
            detail,
            // Directories sort below files: the file is usually what is wanted.
            sort_text: format!("{}{label}", u8::from(entry.is_directory)),
            kind: if entry.is_directory {
                CandidateKind::Directory
            } else {
                CandidateKind::Page
            },
            range,
            label,
        });
    }
    candidates
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p adoc-ls completion`
Expected: PASS, 12 tests.

- [ ] **Step 5: Confirm `filter_by_prefix` does not eat path candidates**

The prefix for `include::../examples/` is `../examples/`, and the label is `../examples/sample.json`, which contains it — so the substring filter keeps it. Run the whole crate to confirm nothing else regressed.

Run: `cargo test -p adoc-ls`
Expected: PASS.

- [ ] **Step 6: Run the full baseline**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo check -p asciidoc-zed-extension --target wasm32-wasip2`
Expected: PASS throughout.

- [ ] **Step 7: Commit**

```bash
git add crates/adoc-ls/src/handlers/completion.rs
git commit -m "feat(ls): complete relative paths in a plain AsciiDoc workspace"
```

---

### Task 8: Corpus probe, documentation, and editor verification

**Files:**
- Create: `<scratchpad>/completion_probe.py` — **never committed, never referenced by a test**
- Modify: `README.md`
- Modify: `docs/prd/DESIGN-Completion_for_References_and_Includes.md` (§5, only if the probe justifies a cap)

**Interfaces:**
- Consumes: the built `adoc-ls` binary and the reference corpus at `/Users/paul/documentation/plus-suite-documentation`.
- Produces: measured candidate counts and latencies; a README that matches shipped behaviour.

**Background:** Fixtures are tiny by design and miss whole classes of real authoring. A previous corpus sweep on this repository found four defects that every fixture passed. The probe exists to answer one open question from the spec: whether an unfiltered `xref:` list needs a cap. The number must come from measurement, and any cap must be logged rather than applied silently.

- [ ] **Step 1: Build the server**

Run: `cargo build --workspace --release`
Expected: success. Note the binary path `target/release/adoc-ls`.

- [ ] **Step 2: Write the probe in the scratchpad**

Write to the session scratchpad directory (not the repository). It speaks LSP over stdio with `Content-Length` framing, opens real corpus pages, and requests completion at a constructed cursor.

```python
#!/usr/bin/env python3
"""Throwaway probe: candidate counts and latency for adoc-ls completion.

Never commit this. It depends on a path that exists only on one machine.
"""
import json
import subprocess
import sys
import time
import urllib.parse
from pathlib import Path

ROOT = Path("/Users/paul/documentation/plus-suite-documentation")
SERVER = Path("target/release/adoc-ls").resolve()

# (label, text appended to the page, expected to be non-empty)
PROBES = [
    ("xref-empty", "\n\nSee xref:", True),
    ("xref-typed", "\n\nSee xref:inst", True),
    ("include-family", "\n\ninclude::", True),
    ("include-partial", "\n\ninclude::partial$", True),
    ("anchor", "\n\nSee <<", True),
    # The control: prose is not a completion context, so zero here means the probe
    # is measuring rather than silently failing.
    ("no-context", "\n\nOrdinary prose ", False),
]


def send(pipe, payload):
    body = json.dumps(payload).encode()
    pipe.write(f"Content-Length: {len(body)}\r\n\r\n".encode())
    pipe.write(body)
    pipe.flush()


def read(pipe):
    length = 0
    while True:
        line = pipe.readline()
        if not line or line in (b"\r\n", b"\n"):
            break
        if line.lower().startswith(b"content-length:"):
            length = int(line.split(b":")[1])
    return json.loads(pipe.read(length)) if length else None


def uri(path):
    return "file://" + urllib.parse.quote(str(path))


def main():
    server = subprocess.Popen(
        [str(SERVER), "--stdio"], stdin=subprocess.PIPE, stdout=subprocess.PIPE
    )
    send(server.stdin, {
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"rootUri": uri(ROOT), "capabilities": {},
                   "workspaceFolders": [{"uri": uri(ROOT), "name": "corpus"}]},
    })
    read(server.stdout)
    send(server.stdin, {"jsonrpc": "2.0", "method": "initialized", "params": {}})

    pages = sorted(ROOT.glob("src/docs/modules/*/pages/**/*.adoc"))
    print(f"{len(pages)} pages found", file=sys.stderr)
    sample = pages[:: max(1, len(pages) // 40)]

    request_id = 2
    for label, suffix, expect_items in PROBES:
        counts, millis = [], []
        for page in sample:
            text = page.read_text(encoding="utf-8", errors="replace") + suffix
            send(server.stdin, {
                "jsonrpc": "2.0", "method": "textDocument/didOpen",
                "params": {"textDocument": {"uri": uri(page), "languageId": "asciidoc",
                                            "version": 1, "text": text}},
            })
            read(server.stdout)  # diagnostics

            line = text.count("\n")
            character = len(text.split("\n")[-1])
            started = time.perf_counter()
            send(server.stdin, {
                "jsonrpc": "2.0", "id": request_id, "method": "textDocument/completion",
                "params": {"textDocument": {"uri": uri(page)},
                           "position": {"line": line, "character": character}},
            })
            reply = read(server.stdout)
            millis.append((time.perf_counter() - started) * 1000)
            request_id += 1

            result = (reply or {}).get("result") or {}
            counts.append(len(result.get("items", [])))

            send(server.stdin, {
                "jsonrpc": "2.0", "method": "textDocument/didClose",
                "params": {"textDocument": {"uri": uri(page)}},
            })
            read(server.stdout)

        worst = max(counts) if counts else 0
        mean = sum(counts) / len(counts) if counts else 0
        slowest = max(millis) if millis else 0
        print(f"{label:16} items mean={mean:7.1f} max={worst:5d} "
              f"slowest={slowest:6.1f}ms")
        if expect_items and worst == 0:
            print(f"  WARNING: {label} produced nothing — the probe may not be measuring",
                  file=sys.stderr)
        if not expect_items and worst != 0:
            print(f"  WARNING: control {label} produced items — detection is too eager",
                  file=sys.stderr)

    send(server.stdin, {"jsonrpc": "2.0", "id": 9999, "method": "shutdown"})
    read(server.stdout)
    send(server.stdin, {"jsonrpc": "2.0", "method": "exit"})
    server.wait(timeout=10)


if __name__ == "__main__":
    main()
```

- [ ] **Step 3: Run the probe**

Run: `python3 <scratchpad>/completion_probe.py`
Expected: one line per probe. The `no-context` control **must** report `max=0`; if it does not, detection is too eager and that is a bug to fix before reading any other number. If any probe expected to produce items reports `max=0`, the probe is not measuring — fix that before drawing conclusions.

- [ ] **Step 4: Decide on a cap from the measurement**

If `xref-empty` reports a mean well inside a few hundred items and the slowest response is comfortably interactive, add no cap and record the measured numbers in §5 of the design document, replacing the open question with the evidence. If it is not, add a cap in `completion_at_offset`, `log` the number of candidates dropped, state the cap and its justification in §5, and add a unit test asserting the cap applies and is reported. Do not add a silent truncation.

- [ ] **Step 5: Verify in Zed**

Run: `cargo install --path crates/adoc-ls --force`, then reload the repository as a Zed dev extension and open an AsciiDoc file in an Antora workspace. Type `xref:`, `include::`, `include::partial$`, `<<`, and `xref:page.adoc#`. Confirm items appear, filter as you type, and insert a valid target. This is the only way to check that the trigger characters and client-side filtering behave as assumed.

- [ ] **Step 6: Update the README**

In `README.md`, remove the completion clause from "Not implemented yet" so it reads:

```markdown
- Rename and references. The only code action is the preview command above.
```

and add to the "Implemented" list, after the Antora navigation entry:

```markdown
- Completion for `xref:` targets, `include::` targets, `image:` targets, and anchors.
  Inside an Antora module the current module's pages are offered as bare IDs and other
  modules' pages module-qualified, `include::` offers the family prefixes and then that
  family's resources, and anchors come from the file a target names. In a plain workspace
  the same constructs complete relative paths, including non-AsciiDoc include targets.
```

- [ ] **Step 7: Run the full baseline one last time**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo run -p adoc-ls -- --version`
Expected: PASS throughout.

- [ ] **Step 8: Commit**

```bash
git add README.md docs/prd/DESIGN-Completion_for_References_and_Includes.md
git commit -m "docs: record completion behaviour and corpus measurements"
```

---

## Self-Review

**Spec coverage:**

| Spec section | Task |
|---|---|
| §3.1 Context detection | Task 1 |
| §3.2 Enumeration | Tasks 2 and 3 |
| §3.3 Candidate assembly — anchors | Task 4 |
| §3.3 Candidate assembly — Antora pages, families, images | Task 6 |
| §3.3 Path completer | Tasks 3 (`list_directory`) and 7 |
| §3.4 LSP surface | Task 5 |
| §4 Error handling | Tasks 4 (`completion_at_offset` contract), 3 (`list_directory` on an unreadable directory), 7 |
| §5 List size | Task 5 (`is_incomplete`), Task 4 (`filter_by_prefix`), Task 8 steps 3–4 (the cap decision) |
| §6 Testing | Every task; corpus probe and Zed check in Task 8 |
| §7 Definition of done | Task 8 steps 5–7 |

**Placeholder scan:** no "TBD" or "handle edge cases" steps; every code step carries the code. The one genuinely open value — whether a cap is needed — is deferred to a measurement with both branches spelled out, rather than left vague.

**Type consistency:** `CompletionContext`/`CompletionKind` as defined in Task 1 are consumed unchanged in Task 4. `Candidate`/`CandidateKind` as defined in Task 4 are consumed unchanged in Tasks 5, 6 and 7. `resources_in`/`modules_of` signatures in Task 2 match their call sites in Task 6. `anchors_in`/`files_under`/`list_directory`/`DirectoryEntry` in Task 3 match their call sites in Tasks 4 and 7. `reference_target_path` in Task 4 is used only in Task 4.

**Rough edges deliberately left in:** none. Task 6's `antora_include_candidates` binds `context` only to prove the file sits in an Antora module before offering Antora candidates; the discard is commented so it does not read as a mistake.
