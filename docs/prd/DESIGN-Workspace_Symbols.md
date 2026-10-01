# Design: Workspace Symbols

| | |
|---|---|
| Author | Paul Snow |
| Version | 0.0.0 |
| Date | 2026-10-01 |
| Status | Proposed |
| Supersedes | Nothing. Implements PRD Appendix B item 8 and the "Workspace Symbols" part of PRD §17. |

## 1. Purpose

Let an author find a heading, an explicit anchor, or an attribute declaration anywhere in the
workspace from Zed's project symbol search, through `workspace/symbol`.

## 2. Constraint: direct maps, not scans

The index answers every query through direct maps and never scans every document. A symbol
search is inherently workspace-wide, so the index keeps a word map whose ordered range scans
cost in proportion to the answer, as the completion enumerators do. The price is recall:
matching is by word prefix, so a fuzzy query such as `athn` for "Authentication" finds
nothing. Zed re-ranks what it receives with its own fuzzy matcher, so prefix queries — the
common case — behave as authors expect.

## 3. Symbols

| Source | Name | Kind | Range |
|---|---|---|---|
| Document title (`= Guide`) | title text | `Title` | title selection range |
| Section heading | heading text | `Section` | section selection range |
| Explicit anchor (`[[id]]`, `[#id]`) | the id | `Anchor` | anchor range |
| Attribute declaration (`:name: value`) | the name | `Attribute` | declaration range |

Unset forms (`:name!:`, `:!name:`) declare nothing and are skipped. Every symbol carries the
declaring document's title, when it has one, as its container.

Include tags (`tag::name[]`), which PRD §17 lists, are not recorded by the parser and are out
of scope.

## 4. Architecture

### 4.1 Domain types (`adoc-index/src/symbols.rs`)

```rust
pub enum WorkspaceSymbolKind { Title, Section, Anchor, Attribute }

pub struct WorkspaceSymbol {
    pub name: String,
    pub kind: WorkspaceSymbolKind,
    pub path: PathBuf,
    pub range: SourceRange,
    pub container: Option<String>,
}
```

They live in `adoc-index`, so no `lsp-types` leak below `adoc-ls`. A crate-private
`document_symbols(path, document)` produces a file's symbols in a fixed order: title,
sections, anchors, attributes.

### 4.2 Words

A name's words are its lower-cased runs of letters and digits: "Authentication Flow" →
`authentication`, `flow`; `page-component-name` → `page`, `component`, `name`. Queries are
split the same way, so punctuation and case never matter.

### 4.3 Index maps

```rust
symbols: BTreeMap<PathBuf, Vec<WorkspaceSymbol>>,       // per file, in document order
symbol_words: BTreeSet<(String, PathBuf, usize)>,        // (word, file, position in its Vec)
```

- **`replace`** builds the file's symbols, inserts one `symbol_words` entry per word per
  symbol, and stores the vector.
- **`remove`** takes the file's stored vector and removes exactly its entries; nothing else
  in the map is visited.

### 4.4 Query

`symbols_matching(query) -> Vec<&WorkspaceSymbol>`:

1. Split the query into words; an empty result returns no symbols, rather than the whole
   workspace.
2. Range-scan `symbol_words` from `(first, "", 0)` while the word starts with the first
   query word, collecting `(path, position)` into a set, so a symbol matching on several of
   its words appears once.
3. Keep symbols for which every remaining query word is a prefix of one of the symbol's
   words.

Results are in `(path, position)` order. No cap is applied: the completion design measured
lists of several hundred items as comfortably interactive, and Zed virtualises the list.

### 4.5 LSP surface (`adoc-ls`)

- Advertise `workspaceSymbolProvider: true`.
- Answer `workspace/symbol` with `WorkspaceSymbolResponse::Flat(Vec<SymbolInformation>)`:
  `Title` → `FILE` and `Section` → `NAMESPACE` (as the document outline uses), `Anchor` →
  `KEY`, `Attribute` → `VARIABLE` (as attribute completion uses).
- Convert each range with the target file's indexed text — the open buffer when the file is
  open — using the negotiated position encoding.

## 5. Testing

- `symbols.rs`: word splitting; one document yields each kind with its container, and unset
  attributes are skipped.
- `index.rs`: prefix of any word finds a symbol, case-insensitively; every query word must
  match; a symbol matching on two words appears once; replacing and removing a file forget
  its symbols; an empty or punctuation-only query returns nothing.
- `capabilities.rs`: the provider is advertised.
- `protocol.rs`: a `workspace/symbol` request after `didOpen` returns the section with kind
  `NAMESPACE`, its container, URI and line.

## 6. Definition of done

- The behaviour above, covered by the tests in §5.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` and the `wasm32-wasip2` check pass.
- `README.md` lists workspace symbols.

## 7. Out of scope

Fuzzy matching, include tags, find references, and ranking beyond Zed's own.
