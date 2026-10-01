# Workspace Symbols Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Answer `workspace/symbol` with headings, explicit anchors and attribute declarations from across the workspace, matched by word prefix.

**Architecture:** `adoc-index` derives each file's symbols on `replace`, keeps them per file plus a `(word, path, position)` set, and answers queries with an ordered range scan on the first query word. `adoc-ls` advertises the provider and maps results to `SymbolInformation`.

**Tech Stack:** Rust workspace (`adoc-core`, `adoc-index`, `adoc-ls`, `lsp-types` 0.97); no new dependencies.

**Spec:** `docs/prd/DESIGN-Workspace_Symbols.md`

## Global Constraints

- The index answers queries through direct maps; never scan every document or every map entry per request.
- No `lsp-types` outside `adoc-ls`. No new dependencies.
- Paths go through `normalize_path`; never canonicalise.
- Always pass `--workspace` or `-p <crate>` to cargo.
- British spelling in comments, messages and docs.

---

### Task 1: Symbol extraction and word splitting

**Files:**
- Create: `crates/adoc-index/src/symbols.rs`
- Modify: `crates/adoc-index/src/lib.rs` (module + exports)

**Interfaces:**
- Produces: `pub enum WorkspaceSymbolKind { Title, Section, Anchor, Attribute }`; `pub struct WorkspaceSymbol { name: String, kind: WorkspaceSymbolKind, path: PathBuf, range: SourceRange, container: Option<String> }`; `pub(crate) fn document_symbols(path: &Path, document: &Document) -> Vec<WorkspaceSymbol>`; `pub(crate) fn words(text: &str) -> impl Iterator<Item = String> + '_`.

- [ ] **Step 1: Write the module with stubs and failing tests**

`crates/adoc-index/src/symbols.rs`:

```rust
//! Named things in a document that a workspace-wide search can find.

use std::path::{Path, PathBuf};

use adoc_core::{Document, SourceRange};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceSymbolKind {
    Title,
    Section,
    Anchor,
    Attribute,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceSymbol {
    pub name: String,
    pub kind: WorkspaceSymbolKind,
    pub path: PathBuf,
    /// The span to select when the symbol is chosen.
    pub range: SourceRange,
    /// The title of the document that declares the symbol.
    pub container: Option<String>,
}

pub(crate) fn document_symbols(path: &Path, document: &Document) -> Vec<WorkspaceSymbol> {
    let _ = (path, document);
    Vec::new()
}

pub(crate) fn words(text: &str) -> impl Iterator<Item = String> + '_ {
    let _ = text;
    std::iter::empty()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use adoc_parser::parse;

    use super::{document_symbols, words, WorkspaceSymbolKind};

    #[test]
    fn splits_names_into_lower_cased_words() {
        assert_eq!(
            words("Authentication Flow").collect::<Vec<_>>(),
            ["authentication", "flow"]
        );
        assert_eq!(
            words("page-component-name").collect::<Vec<_>>(),
            ["page", "component", "name"]
        );
        assert_eq!(
            words("API_v2: Setup").collect::<Vec<_>>(),
            ["api", "v2", "setup"]
        );
    }

    #[test]
    fn collects_each_kind_of_symbol_with_its_container() {
        let text =
            "= Guide\n:product-name: Widget\n:draft!:\n\n[[auth-flow]]\n== Authentication Flow\n";
        let document = parse("file:///docs/guide.adoc", text).document;

        let symbols = document_symbols(Path::new("/docs/guide.adoc"), &document);

        assert_eq!(
            symbols
                .iter()
                .map(|symbol| (symbol.name.as_str(), symbol.kind))
                .collect::<Vec<_>>(),
            [
                ("Guide", WorkspaceSymbolKind::Title),
                ("Authentication Flow", WorkspaceSymbolKind::Section),
                ("auth-flow", WorkspaceSymbolKind::Anchor),
                ("product-name", WorkspaceSymbolKind::Attribute),
            ]
        );
        assert!(symbols
            .iter()
            .all(|symbol| symbol.container.as_deref() == Some("Guide")));
    }
}
```

`crates/adoc-index/src/lib.rs`: add `mod symbols;` and `pub use symbols::{WorkspaceSymbol, WorkspaceSymbolKind};`.

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p adoc-index symbols`
Expected: both tests FAIL on assertions (empty results).

- [ ] **Step 3: Implement**

```rust
/// A file's symbols in a fixed order: title, sections, anchors, attributes.
pub(crate) fn document_symbols(path: &Path, document: &Document) -> Vec<WorkspaceSymbol> {
    let container = document.title.as_ref().map(|title| title.text.clone());
    let symbol = |name: &str, kind, range| WorkspaceSymbol {
        name: name.to_owned(),
        kind,
        path: path.to_path_buf(),
        range,
        container: container.clone(),
    };

    let mut symbols = Vec::new();
    if let Some(title) = &document.title {
        symbols.push(symbol(
            &title.text,
            WorkspaceSymbolKind::Title,
            title.selection_range,
        ));
    }
    symbols.extend(document.sections.iter().map(|section| {
        symbol(
            &section.title,
            WorkspaceSymbolKind::Section,
            section.selection_range,
        )
    }));
    symbols.extend(
        document
            .anchors
            .iter()
            .map(|anchor| symbol(&anchor.id, WorkspaceSymbolKind::Anchor, anchor.range)),
    );
    // `:name!:` and `:!name:` unset an attribute rather than declaring one.
    symbols.extend(
        document
            .attributes
            .iter()
            .filter(|attribute| {
                !attribute.name.starts_with('!') && !attribute.name.ends_with('!')
            })
            .map(|attribute| {
                symbol(
                    &attribute.name,
                    WorkspaceSymbolKind::Attribute,
                    attribute.range,
                )
            }),
    );
    symbols
}

/// The lower-cased runs of letters and digits in `text`, so that neither case nor
/// punctuation affects matching.
pub(crate) fn words(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p adoc-index symbols`
Expected: PASS. (`dead_code` warnings for the crate-private functions are expected until Task 2 uses them; do not commit until Task 2 if clippy is run with `-D warnings`.)

---

### Task 2: Symbol maps and query in `WorkspaceIndex`

**Files:**
- Modify: `crates/adoc-index/src/index.rs`

**Interfaces:**
- Consumes: `document_symbols`, `words`, `WorkspaceSymbol` (Task 1).
- Produces: `WorkspaceIndex::symbols_matching(&self, query: &str) -> Vec<&WorkspaceSymbol>`.

- [ ] **Step 1: Write the failing tests** — append to `mod tests` in `index.rs`:

```rust
    fn names(symbols: Vec<&crate::WorkspaceSymbol>) -> Vec<&str> {
        symbols.iter().map(|symbol| symbol.name.as_str()).collect()
    }

    const GUIDE: &str = "= Guide\n\n[[auth-flow]]\n== Authentication Flow\n\n== Authentication Setup\n";

    #[test]
    fn finds_symbols_by_the_prefix_of_any_word() {
        let mut index = WorkspaceIndex::new();
        index.index_source("docs/guide.adoc", GUIDE);

        assert_eq!(
            names(index.symbols_matching("auth")),
            ["Authentication Flow", "Authentication Setup", "auth-flow"]
        );
        assert_eq!(
            names(index.symbols_matching("FLOW")),
            ["Authentication Flow", "auth-flow"]
        );
    }

    #[test]
    fn requires_every_query_word_to_match() {
        let mut index = WorkspaceIndex::new();
        index.index_source("docs/guide.adoc", GUIDE);

        assert_eq!(
            names(index.symbols_matching("auth set")),
            ["Authentication Setup"]
        );
        assert!(index.symbols_matching("auth missing").is_empty());
    }

    #[test]
    fn returns_a_symbol_once_when_several_of_its_words_match() {
        let mut index = WorkspaceIndex::new();
        index.index_source("docs/guide.adoc", "== Flow Flowchart\n");

        assert_eq!(names(index.symbols_matching("flow")), ["Flow Flowchart"]);
    }

    #[test]
    fn forgets_the_symbols_of_replaced_and_removed_files() {
        let path = PathBuf::from("docs/guide.adoc");
        let mut index = WorkspaceIndex::new();
        index.index_source(&path, GUIDE);

        index.index_source(&path, "== Overview\n");
        assert!(index.symbols_matching("auth").is_empty());
        assert_eq!(names(index.symbols_matching("over")), ["Overview"]);

        index.remove(&path);
        assert!(index.symbols_matching("over").is_empty());
    }

    #[test]
    fn returns_nothing_for_an_empty_query() {
        let mut index = WorkspaceIndex::new();
        index.index_source("docs/guide.adoc", GUIDE);

        assert!(index.symbols_matching("").is_empty());
        assert!(index.symbols_matching(" -- ").is_empty());
    }
```

Add a stub so they compile:

```rust
    #[must_use]
    pub fn symbols_matching(&self, query: &str) -> Vec<&WorkspaceSymbol> {
        let _ = query;
        Vec::new()
    }
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p adoc-index symbol`
Expected: the "finds", "requires", "returns a symbol once" and "forgets" tests FAIL on assertions; the empty-query test passes trivially as a guard.

- [ ] **Step 3: Implement**

Imports: `use crate::symbols::{document_symbols, words, WorkspaceSymbol};`.

Struct fields:

```rust
    /// Each file's symbols in document order.
    symbols: BTreeMap<PathBuf, Vec<WorkspaceSymbol>>,
    /// `(word, file, position in that file's symbols)`, so a query reaches its matches by an
    /// ordered range scan rather than by visiting every symbol.
    symbol_words: BTreeSet<(String, PathBuf, usize)>,
```

In `replace`, after the includers loop:

```rust
        let symbols = document_symbols(&path, &document);
        for (position, symbol) in symbols.iter().enumerate() {
            for word in words(&symbol.name) {
                self.symbol_words.insert((word, path.clone(), position));
            }
        }
        self.symbols.insert(path.clone(), symbols);
```

In `remove`, before `let removed = self.files.remove(&path)?;`:

```rust
        if let Some(symbols) = self.symbols.remove(&path) {
            for (position, symbol) in symbols.iter().enumerate() {
                for word in words(&symbol.name) {
                    self.symbol_words.remove(&(word, path.clone(), position));
                }
            }
        }
```

Replace the stub:

```rust
    /// Symbols with a word starting with each word of `query`, in file and document order.
    ///
    /// An empty query finds nothing rather than every symbol in the workspace.
    #[must_use]
    pub fn symbols_matching(&self, query: &str) -> Vec<&WorkspaceSymbol> {
        let mut query_words = words(query);
        let Some(first) = query_words.next() else {
            return Vec::new();
        };
        let rest: Vec<String> = query_words.collect();

        let hits: BTreeSet<(&Path, usize)> = self
            .symbol_words
            .range((first.clone(), PathBuf::new(), 0)..)
            .take_while(|(word, _, _)| word.starts_with(&first))
            .map(|(_, path, position)| (path.as_path(), *position))
            .collect();

        hits.into_iter()
            .filter_map(|(path, position)| self.symbols.get(path)?.get(position))
            .filter(|symbol| {
                rest.iter().all(|wanted| {
                    words(&symbol.name).any(|word| word.starts_with(wanted.as_str()))
                })
            })
            .collect()
    }
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p adoc-index` then `cargo clippy -p adoc-index --all-targets -- -D warnings`
Expected: all pass, no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/adoc-index/src
git commit -m "Index headings, anchors and attributes as workspace symbols"
```

---

### Task 3: `workspace/symbol` in `adoc-ls`

**Files:**
- Modify: `crates/adoc-ls/src/capabilities.rs` (provider + test)
- Modify: `crates/adoc-ls/src/protocol.rs` (route, response, test)

**Interfaces:**
- Consumes: `WorkspaceIndex::symbols_matching`, `adoc_index::WorkspaceSymbolKind` (Task 2).

- [ ] **Step 1: Write the failing tests**

`capabilities.rs` tests:

```rust
    #[test]
    fn advertises_workspace_symbols() {
        let capabilities = server_capabilities(PositionEncoding::Utf16);

        assert_eq!(
            capabilities.workspace_symbol_provider,
            Some(OneOf::Left(true))
        );
    }
```

`protocol.rs` tests (reuses the existing `open` helper and `PAGE` constant):

```rust
    #[test]
    fn answers_workspace_symbol_requests_from_the_index() {
        let mut server = ProtocolServer::new(PositionEncoding::Utf16);
        open(&mut server, PAGE, "= Guide\n\n== Authentication Flow\n");

        let Some(WorkspaceSymbolResponse::Flat(symbols)) =
            server.workspace_symbol_response(WorkspaceSymbolParams {
                query: "auth".to_owned(),
                ..WorkspaceSymbolParams::default()
            })
        else {
            panic!("expected a flat symbol list");
        };

        assert_eq!(symbols.len(), 1, "{symbols:?}");
        let symbol = &symbols[0];
        assert_eq!(symbol.name, "Authentication Flow");
        assert_eq!(symbol.kind, SymbolKind::NAMESPACE);
        assert_eq!(symbol.container_name.as_deref(), Some("Guide"));
        assert_eq!(symbol.location.uri.as_str(), PAGE);
        assert_eq!(symbol.location.range.start.line, 2);
    }
```

Add `SymbolKind, WorkspaceSymbolParams, WorkspaceSymbolResponse` to the test module's `lsp_types` imports, and a stub on `ProtocolServer`:

```rust
    fn workspace_symbol_response(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Option<WorkspaceSymbolResponse> {
        let _ = params;
        None
    }
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p adoc-ls --lib workspace_symbol`
Expected: both FAIL (provider is `None`; stub returns `None`).

- [ ] **Step 3: Implement**

`capabilities.rs`: `workspace_symbol_provider: Some(OneOf::Left(true)),` after `hover_provider`.

`protocol.rs`: import `WorkspaceSymbolRequest` (request), `SymbolInformation, WorkspaceSymbolParams, WorkspaceSymbolResponse` (types) and `adoc_index::WorkspaceSymbolKind`. Route in `handle_request`:

```rust
            WorkspaceSymbolRequest::METHOD => self
                .request_response::<WorkspaceSymbolParams, _>(request, |params| {
                    self.workspace_symbol_response(params)
                }),
```

Replace the stub:

```rust
    #[allow(deprecated)]
    fn workspace_symbol_response(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Option<WorkspaceSymbolResponse> {
        let symbols = self
            .state
            .index
            .symbols_matching(&params.query)
            .into_iter()
            .filter_map(|symbol| {
                // The indexed text is the open buffer when the file is open.
                let text = &self.state.index.file(&symbol.path)?.document.text;
                Some(SymbolInformation {
                    name: symbol.name.clone(),
                    kind: match symbol.kind {
                        WorkspaceSymbolKind::Title => SymbolKind::FILE,
                        WorkspaceSymbolKind::Section => SymbolKind::NAMESPACE,
                        WorkspaceSymbolKind::Anchor => SymbolKind::KEY,
                        WorkspaceSymbolKind::Attribute => SymbolKind::VARIABLE,
                    },
                    tags: None,
                    deprecated: None,
                    location: Location::new(
                        path_to_uri(&symbol.path)?,
                        self.encoding.range(text, symbol.range)?,
                    ),
                    container_name: symbol.container.clone(),
                })
            })
            .collect();
        Some(WorkspaceSymbolResponse::Flat(symbols))
    }
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p adoc-ls`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add crates/adoc-ls/src
git commit -m "Answer workspace symbol requests"
```

---

### Task 4: Documentation and verification

- [ ] **Step 1: README** — add to the "Implemented" list, after the hover bullet:

```markdown
- Workspace symbols: Zed's project symbol search finds document titles, section headings,
  explicit anchors and attribute declarations across the workspace by the start of any word
  in their names, so `auth` and `flow` both find "Authentication Flow".
```

- [ ] **Step 2: Full verification**

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p asciidoc-zed-extension --target wasm32-wasip2
cargo run -p adoc-ls -- --version
```

- [ ] **Step 3: Commit**

```bash
git add README.md docs/prd/DESIGN-Workspace_Symbols.md docs/superpowers/plans/2026-10-01-workspace-symbols.md
git commit -m "Document workspace symbols"
```
