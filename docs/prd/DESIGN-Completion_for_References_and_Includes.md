# Design: Completion for References and Includes

| | |
|---|---|
| Author | Paul Snow |
| Version | 0.0.0 |
| Date | 2026-08-20 |
| Status | Proposed |
| Supersedes | Nothing. Implements PRD Appendix B items 1–4, and extends items 5–6 no further. |

## 1. Purpose

Give `adoc-ls` a `textDocument/completion` provider covering the five places an author
types a reference to somewhere else in the workspace:

| Construct | Completed |
|---|---|
| `xref:<cursor>` | Antora page IDs, or relative paths outside Antora |
| `xref:target#<cursor>` | anchors declared in the target file |
| `<<<cursor>` | anchors declared in the current file |
| `include::<cursor>` | Antora family resources (`partial$`, `example$`, …), or relative paths |
| `image:<cursor>` / `image::<cursor>` | `image$` resources, or relative paths |

Nothing else in Appendix B is in scope. Hover, find references, workspace symbols,
attribute completion, and `nav.adoc` intelligence remain future work.

## 2. Constraints inherited from the existing design

Three properties of the current code shape this design and must survive it.

- **The index must not be scanned wholesale.** `CLAUDE.md` states that lookups go through
  direct maps and that no feature may scan every document. Completion needs *enumeration*,
  which is a different operation from the point lookups the index was built for, so the
  enumerators below are all ordered-range scans whose cost is proportional to the answer
  rather than to the workspace.
- **The parser records only complete macros.** `parser::tests::tolerates_incomplete_constructs`
  asserts that `xref:unfinished[` produces no reference. Completion therefore cannot reuse
  `document.references`; it needs its own scanner over half-typed text.
- **Diagnostics and navigation are conservative.** Completion follows suit: every failure
  yields an empty list, never an LSP error and never a guess.

## 3. Architecture

```
adoc-parser   completion_context(text, offset) -> Option<CompletionContext>
adoc-antora   AntoraCatalog::resources_in, ::modules_of
adoc-index    WorkspaceIndex::anchors_in, workspace::list_directory
adoc-ls       handlers/completion.rs  (assembly)  →  protocol.rs  (lsp-types)
```

No new crate. Each piece lands beside the code that already owns its concern:
context detection beside `find_references`, enumeration beside the maps being ranged over,
assembly beside `definition.rs`, which already resolves the same targets.

### 3.1 Context detection (`adoc-parser`)

```rust
pub fn completion_context(text: &str, offset: usize) -> Option<CompletionContext>

pub struct CompletionContext {
    pub kind: CompletionKind,
    pub prefix: String,      // construct start → cursor: what the author has typed
    pub range: SourceRange,  // the span a completion replaces
}

pub enum CompletionKind {
    XrefTarget,
    XrefAnchor { target: String }, // empty target means the current file
    IncludeTarget,
    ImageTarget,
    LocalAnchor,
}
```

The function runs the same block-state machine as `parse_document` — `active_delimiter`
and `COMMENT_DELIMITER` — over the lines preceding the cursor, so block behaviour cannot
drift from the parser's:

- inside a comment block (`////`), no context is returned at all;
- inside any other delimited block, only `IncludeTarget` is returned, mirroring
  Asciidoctor's expansion of includes inside `[source]`, listing, literal and passthrough
  blocks;
- elsewhere, every kind is available.

On the cursor's line it scans backwards for the nearest construct the cursor still sits
inside, reusing `is_boundary` so that `\xref:` remains escaped and `myxref:` does not
trigger. The cursor counts as inside the target only while it is before the macro's `[`
and before any whitespace; for `<<` it must be before `>>` or `,`. Every other position
returns `None`.

`range` spans from the start of the target to the cursor. A completion is therefore a
plain `TextEdit`, which avoids negotiating `InsertReplaceEdit` client capability. The
accepted cost is that completing in the middle of an existing target appends rather than
replacing the tail.

### 3.2 Enumeration

```rust
// adoc-antora
pub fn resources_in(&self, component: &str, version: Option<&str>,
                    module: &str, family: ResourceFamily) -> impl Iterator<Item = &AntoraResource>
pub fn modules_of(&self, component: &str, version: Option<&str>) -> impl Iterator<Item = &Module>

// adoc-index
pub fn anchors_in(&self, path: &Path) -> impl Iterator<Item = (&str, &AnchorLocation)>
```

Each is a `BTreeMap` range followed by `take_while`. They are correct because the derived
`Ord` on `AntoraCoordinate` orders fields *component → version → module → family →
relative_path*, so every resource in one module-family is contiguous, and `AnchorKey`
orders *path → id*. No new index or auxiliary map is introduced.

### 3.3 Candidate assembly (`adoc-ls/src/handlers/completion.rs`)

| Context | Inside an Antora module | Outside Antora |
|---|---|---|
| `XrefTarget` | current module's pages as bare IDs, sorted first; then every other module's pages as `module:page.adoc` | path completer |
| `IncludeTarget` | the family prefixes (`partial$`, `example$`, `image$`, `attachment$`, `page$`) until `$` is typed; that family's resources afterwards | path completer |
| `ImageTarget` | `image$` resources | path completer |
| `XrefAnchor { target }` | `target` resolved through the same order `definition.rs` uses, then `anchors_in` | same |
| `LocalAnchor` | `anchors_in` on the current file | same |

The index registers several ids per section — the title, Antora's `getting-started` form,
Asciidoctor's `_getting_started` form, an alphanumeric form — so that resolution is forgiving
about how a reference is written. Completion collapses them to one candidate per section,
preferring the explicit anchor where one is declared and Antora's generated form where none is.

Ordering is carried by `sortText`, not by list position: `0` prefixes the current module's
bare IDs and `1` prefixes module-qualified IDs, so the current module ranks first while the
client remains free to filter.

Every item sets `detail` to the target's document title, read directly from
`index.file(path)`. Because the detail is available without further work, the server
advertises `resolve_provider: false` and needs no `completionItem/resolve` round-trip.

**The path completer** is the only component that touches the filesystem. It splits the
typed prefix at the last `/`, resolves that directory against the current document, then
lists it with one shallow `read_dir`. Reading the directory rather than the index is what
makes `query.sql`, `diagram.png` and other non-AsciiDoc include targets completable at all,
since the index holds only `.adoc` files. Dotfiles and `DEFAULT_IGNORED_DIRECTORIES` are
skipped. It lives
in `adoc-index::workspace`, which already owns `is_asciidoc_path`, the ignored-directory
list and `collect_asciidoc_files`. Because it resolves relative to the typed prefix,
`../shared/` works without special handling.

### 3.4 LSP surface

`capabilities.rs` adds:

```rust
completion_provider: Some(CompletionOptions {
    trigger_characters: Some(vec![":".into(), "$".into(), "#".into(), "/".into(), "<".into()]),
    resolve_provider: Some(false),
    ..CompletionOptions::default()
}),
```

`protocol.rs` adds one dispatch arm beside `GotoDefinition`, converting the cursor position
through the existing `position.rs` encoding negotiation and mapping each item's
`SourceRange` back through the same path.

Trigger characters fire indiscriminately — `:` also fires on an attribute line such as
`:toc:`. That is intended. `completion_context` returns `None` there and the response is an
empty list, so detection is the guard and the trigger set is only a hint about when to ask.

The Zed extension is unchanged. Completion reaches the editor through capability
negotiation alone, so `extension.toml` and `src/lib.rs` are untouched.

## 4. Error handling

Every failure path returns an empty `CompletionList` rather than an LSP error: document not
open, cursor beyond the end of the text, no resolvable Antora context, unreadable
directory, or a `XrefAnchor` target that resolves nowhere. This matches
`definition_at_offset` returning `None` and the diagnostics rule that a false positive is
worse than silence.

## 5. List size

Responses set `isIncomplete: true`, so the client re-queries as the prefix grows and the
server can filter against the typed prefix on each request.

No cap is placed on the candidate list. The corpus probe (§6) measured the real reference
site — `/Users/paul/documentation/plus-suite-documentation`, 541 pages across 19 modules in
one component — driving the built `adoc-ls` binary over stdio, opening a sample of every
14th page (40 pages), and requesting completion at five constructed contexts plus a
no-context control:

| context           | items (mean) | items (max) | slowest response |
|--------------------|-------------:|------------:|------------------:|
| `xref:` (empty)    |        554.0 |         554 |             54.0ms |
| `xref:index`       |         53.0 |          53 |              6.3ms |
| `include::`        |          5.0 |           5 |              1.2ms |
| `include::partial$`|         32.2 |          74 |              7.8ms |
| `<<` (anchor)      |         15.4 |         233 |             22.8ms |
| no context         |          0.0 |           0 |              0.6ms |

The no-context control reported zero, confirming the probe measures rather than passing
vacuously. The unfiltered `xref:` case is the worst one, at 554 candidates — the whole
component's page list, close to the 541 pages predicted here before measurement — and it is
still the slowest response recorded, at 54ms. That is comfortably interactive: nowhere near
the ~100ms threshold at which a response starts to feel laggy, let alone one that would need
a loading indicator. As soon as a character is typed the response narrows sharply (554 to 53
for a five-letter prefix that matches the corpus's most common page basename) because
`isIncomplete: true` makes the client re-query and `filter_by_prefix` runs again on the
longer prefix each time, so the full unfiltered list is only ever shown for a moment before
the first keystroke narrows it. A hand-rolled cap would add complexity — a dropped-item count
to surface, a threshold to justify, a unit test to keep it honest — to solve a problem this
measurement does not show: latency stays sub-frame-rate at the corpus's actual size, and list
rendering at a few hundred items is exactly what LSP clients, including Zed's completion
menu, are built to virtualise. **Decision: no cap.** If a future corpus is materially larger
than this one and either count or latency degrades, this measurement is the baseline to
compare against.

## 6. Testing

`tests/fixtures/antora-single-component` already contains two modules (`ROOT` and
`security`) and one resource per family, which is sufficient for module-qualified ordering
and family prefixes. No new fixture is required.

- **`adoc-parser`** — a unit test per `CompletionKind`, plus the cases that make detection
  trustworthy: a comment block yields `None`; a `[source]` block yields `IncludeTarget`
  only; `\xref:` stays escaped; `myxref:` does not trigger; a cursor after `[` yields
  `None`; and `xref:page.adoc#` yields `XrefAnchor`.
- **`adoc-antora` and `adoc-index`** — each enumerator returns exactly its own range and
  nothing adjacent; `resources_in(ROOT, Partial)` must not leak `security`'s partials.
- **`adoc-ls`** — `completion_at_offset` over the fixture: bare IDs ranked before
  module-qualified ones, family prefixes on `include::`, anchors after `#`.
- **`crates/adoc-ls/tests/stdio.rs`** — a real `textDocument/completion` round-trip through
  the built binary, asserting both that the capability is advertised and that items return.
- **Corpus probe** — a throwaway harness written in the scratchpad, driving the built
  server over stdio against the reference Antora corpus, recording candidate counts and
  response latency per context. It is sanity-checked against a position with no context so
  that a zero means "correctly nothing" rather than "not measuring". It is never committed
  and never referenced from a test.
- **Zed check** — one manual pass confirming that items render and insert in the editor,
  which is the only way to verify trigger characters and client-side filtering behave as
  assumed.

Development follows TDD, consistent with the repository's convention of adding a test
alongside every parser, resolver and handler change.

## 7. Definition of done

- The five contexts above complete inside an Antora module and in a plain workspace.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace` pass.
- `cargo check -p asciidoc-zed-extension --target wasm32-wasip2` still passes.
- The corpus probe reports candidate counts and latency, and any cap introduced as a result
  is recorded here with its measured justification.
- Completion is confirmed working in Zed.
- `README.md` moves completion out of "Not implemented yet" and describes the behaviour.

## 8. Out of scope

Attribute completion (`{page-`), hover, find references, workspace symbols, `antora.yml`
and `nav.adoc` intelligence, cross-component and cross-version candidates, and snippet or
placeholder insertion beyond a plain target string.
