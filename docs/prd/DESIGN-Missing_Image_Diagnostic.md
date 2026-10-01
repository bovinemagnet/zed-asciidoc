# Design: Missing-Image Diagnostic and Includers Map

| | |
|---|---|
| Author | Paul Snow |
| Version | 0.0.0 |
| Date | 2026-10-01 |
| Status | Proposed |
| Supersedes | Nothing. Implements the "Missing image" diagnostic in PRD §18. |

## 1. Purpose

Report a block image whose target does not exist:

```asciidoc
image::missing.png[]
```

as a warning with the stable code `adoc.unresolved-image`, without producing false
positives for images whose resolution depends on a document the current file is included
into.

## 2. The problem with included files

Asciidoctor resolves an include target relative to the file containing the directive, but
an image target relative to the *top-level* document's `imagesdir` (by default its
directory). A file that is included elsewhere therefore cannot judge its own images:

```text
guide.adoc                  include::chapters/setup.adoc[]
chapters/setup.adoc         image::screens/login.png[]   → guide.adoc's dir/screens/login.png
```

Resolving `screens/login.png` against `chapters/` would warn falsely. Antora has the same
property, which the server already respects for `<<anchor>>` diagnostics by not judging
files in the `partial` family.

Knowing that `chapters/setup.adoc` is included requires a reverse lookup. The index offers
only forward lookups today, and scanning every document per request is ruled out by the
index's direct-map constraint. This design adds the reverse map.

## 3. Constraints inherited from the existing design

- **Direct maps only.** The index never scans every document to answer a query.
- **Layering.** `adoc-index` depends on `adoc-parser` and `adoc-core` only; it cannot
  resolve Antora resource IDs. Antora-aware decisions belong in `adoc-ls`.
- **Lexical paths.** Targets are resolved with `normalize_path`, never canonicalised, so
  unsaved and not-yet-existing files work.
- **Conservative diagnostics.** No diagnostic is better than a false positive. Dynamic
  targets produce nothing.

## 4. Architecture

### 4.1 Includers map (`adoc-index`)

`WorkspaceIndex` gains:

```rust
includers: BTreeMap<PathBuf, BTreeSet<PathBuf>>,   // include target → files including it
```

- **`replace(path, document)`** resolves each `include::` target with the existing
  `resolve_include_target(&document, &path, target)` and records `path` against every
  target that resolves. Targets that do not resolve — Antora resource IDs, URLs, and
  attribute references the document does not declare — are not recorded.
- **`remove(path)`** takes the removed `FileEntry`, resolves its include targets again,
  and removes `path` from each of those sets, dropping sets that become empty. Resolution
  is a pure function of the document text and its path, so the second resolution yields
  exactly the targets the first one recorded.
- **Query:** `pub fn includers(&self, path: &Path) -> impl Iterator<Item = &Path>`, keyed
  by `normalize_path(path)`.

The map is keyed by target, so it is independent of indexing order: a file indexed before
the file that includes it is still found once the includer is indexed.

### 4.2 Plain-workspace image resolution (`adoc-index`)

`pub fn resolve_image_target(document, current_path, target) -> Option<PathBuf>`, next to
`resolve_include_target`:

1. Return `None` for a target that is a URL (`://`), a `data:` URI, or contains `$` or `:`
   (an Antora resource ID, which this layer cannot judge).
2. Substitute attributes the document declares, exactly as includes do; return `None` if
   any reference remains unresolved.
3. An absolute target is used as is. Otherwise the base is the document's directory, joined
   with the document's `imagesdir` when it declares one (itself attribute-substituted; an
   unresolvable `imagesdir` returns `None`).

### 4.3 Index-layer diagnostic (`adoc-index`)

`workspace_diagnostics` adds, for each `document.images` entry:

- skip when `index.includers(path)` is non-empty — the file is composed into another;
- skip when `resolve_image_target` returns `None`;
- otherwise warn `adoc.unresolved-image` (`DiagnosticCode::UnresolvedImage`) when the
  resolved path neither exists on disk nor names an indexed file.

### 4.4 Antora override (`adoc-ls/src/handlers/diagnostics.rs`)

Inside an Antora module, image targets are resource IDs whose default family is `image`
and whose path is relative to the module's `images/` directory, so the index layer's
document-relative answer is wrong in both directions. For a file with an Antora context:

1. Drop every `UnresolvedImage` the index layer produced.
2. If the file is in the `partial` family, stop: as with anchors, a partial is judged only
   as part of the page that includes it.
3. Otherwise, for each image not skipped by the URL/`data:`/dynamic rules, parse the
   target with `parse_resource_id` and resolve it against the page's context with
   `family` set to `ResourceFamily::Image`:
   - resolved → nothing;
   - `UnknownComponent` → nothing (published from elsewhere in the playbook, as for xrefs);
   - `UnknownModule` → `adoc.antora.unknown-module`, as for xrefs;
   - `UnknownResource` → `adoc.unresolved-image`.

### 4.5 Shared resolution for hover

`adoc-ls` gains `handlers/images.rs` with
`resolve_image(index, antora, current_path, document, target) -> Option<PathBuf>`:
the Antora resolution of §4.4 when the file has an Antora context, `resolve_image_target`
otherwise. Hover uses it to show where an image points; diagnostics use the same Antora
branch so the two cannot disagree.

## 5. Behaviour summary

| Situation | Result |
|---|---|
| Plain document, image missing | warning |
| Plain document, image present (directly or under declared `imagesdir`) | nothing |
| Plain document included by another indexed file | nothing |
| URL, `data:` URI, undeclared `{attribute}` in target | nothing |
| Antora page, `image$` resource missing in module | warning |
| Antora page, `other-module:x.png` with unknown module | unknown-module warning |
| Antora page, image in a component absent from the workspace | nothing |
| Antora partial | nothing |

## 6. Known limitations

- **Diagnostics are republished only for the document that changed.** Adding an
  `include::` to `guide.adoc` clears `setup.adoc`'s image warnings in the index at once,
  but the editor shows the change only when `setup.adoc` is next edited or reopened. Xref
  and anchor diagnostics already behave this way; cross-file republishing is separate work.
  *Since resolved:* every open document's diagnostics are now republished whenever any
  document is opened, edited or closed.
- **Includes the index cannot resolve are invisible to the map.** A file reached only
  through `include::{undeclared}/x.adoc[]` is treated as top-level and may warn.
- **Antora pages included by other pages** (`include::page$…`) are judged on their own
  module context, because the map holds only plain-path includes.
- **Inline images** (`image:icon.png[]`) are not recorded by the parser and are not
  checked. Extending the parser is separate work.

## 7. Testing

- `adoc-index`, includers map: an include is recorded; re-replacing the includer without
  the directive clears it; removing the includer clears it; indexing order does not matter;
  an attribute-based target the document declares is recorded.
- `adoc-index`, `resolve_image_target`: relative, absolute, declared `imagesdir`,
  undeclared attribute, URL, `data:`, Antora-shaped target.
- `adoc-index`, diagnostics: missing image warns; present image does not; included file is
  skipped.
- `adoc-ls`, diagnostics with the Antora fixture: missing `image$` resource warns; present
  one does not; module-qualified target resolves; unknown component is silent; partial is
  silent; the index layer's document-relative false positive is dropped.
- A small `tests/fixtures/images/` fixture for the plain cases; the existing
  `antora-single-component` fixture (which has `images/architecture.svg`) for Antora.

## 8. Definition of done

- `adoc.unresolved-image` behaves as §5 describes, covered by the tests in §7.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` and the `wasm32-wasip2` check pass.
- `README.md` lists the diagnostic and its include-context rule.

## 9. Out of scope

Inline image checking, cross-file diagnostic republishing, Antora-aware includers, image
Go to Definition, and image completion changes.
