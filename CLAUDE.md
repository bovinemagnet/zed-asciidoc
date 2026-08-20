# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

A Zed extension plus a standalone Rust language server (`adoc-ls`) for AsciiDoc, Asciidoctor, and Antora authoring. `docs/prd/PRD-AsciiDoc_for_Zed_Initial_Implementation_Specification.md` is the implementation source of truth; `README.md` tracks what is and is not implemented yet and must be updated when setup or visible behaviour changes.

`AGENTS.md` holds the repository contribution guidelines and is consistent with this file.

## Commands

```sh
cargo check --workspace
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings   # CI treats warnings as errors
cargo test --workspace
cargo check -p asciidoc-zed-extension --target wasm32-wasip2   # Zed Wasm target
cargo run -p adoc-ls -- --version                              # smoke-test the binary
```

`default-members = ["."]` in `Cargo.toml`, so a bare `cargo test` or `cargo check` only covers the Zed extension crate. **Always pass `--workspace`** (or `-p <crate>`) when working on the engine crates.

Targeted runs:

```sh
cargo test -p adoc-antora                       # one crate
cargo test -p adoc-ls --test stdio              # one integration test file
cargo test --workspace resolves_module_qualified # single test by name substring
```

Load the repository root as a Zed dev extension only after `cargo install --path crates/adoc-ls`; the extension resolves `adoc-ls` from `PATH` and starts it with `--stdio`. Dev-extension builds need rustup with the `wasm32-wasip2` target.

## Architecture

The workspace is layered so that everything except the extension is testable without launching Zed.

```
src/lib.rs (asciidoc-zed-extension, wasm32-wasip2)  →  spawns `adoc-ls --stdio` from PATH
        adoc-ls  →  adoc-index, adoc-antora, adoc-parser, adoc-core
        adoc-index  →  adoc-parser, adoc-core
        adoc-parser →  adoc-core
        adoc-core   →  (no internal deps; no LSP or editor types)
        adoc-render →  (standalone; not yet wired into adoc-ls)
```

- **`adoc-core`** — editor-independent domain types only: `Document`, `LineIndex`, `SourceRange` (byte offsets), `Section`, `Anchor`, `Reference`/`ReferenceKind`, `IncludeDirective`, `Diagnostic`/`DiagnosticCode`. Diagnostic codes are stable strings like `adoc.unresolved-xref-file` and `adoc.antora.unknown-module`.
- **`adoc-parser`** — line-oriented semantic parser (`parse(uri, text) -> ParseResult`), deliberately *not* Tree-sitter. It skips verbatim/delimited blocks, and extracts titles, sections, attributes, anchors, xrefs, includes, and images. `completion_context(text, offset)` runs the same block-state machine to detect which construct the cursor sits inside for `textDocument/completion`. Internal AST/line helpers stay private to the crate.
- **`adoc-index`** — `WorkspaceIndex` with replacement-based updates: `index_roots`, `index_source`, `replace`, `remove`. Lookups go through direct maps (path → `FileEntry`, `(path, anchor id)` → `AnchorLocation`); never add features that scan every document. `workspace.rs` owns extension matching (`.adoc`/`.asciidoc`/`.ad`), the ignored-directory list, and `normalize_path` (lexical, never canonicalising, so unsaved files work).
- **`adoc-antora`** — typed Antora model: descriptor parsing (`antora.yml` via `serde-saphyr`), `AntoraCatalog` of components/modules/resources, `parse_resource_id` for `version@component:module:family$path`, and `AntoraResolver` returning typed `ResolutionError`s. The model already carries component *and* version so multi-component support needs no redesign, even though only same-component resolution is implemented.
- **`adoc-ls`** — the only crate that knows about LSP. `server.rs` parses argv (`--stdio`, `--version`, `--help`) and owns `ServerError`; `protocol.rs` runs the `lsp-server` connection loop and converts core types to `lsp-types`; `state.rs` holds `ServerState` (open documents, index, Antora catalog, descriptor issues); `position.rs` negotiates UTF-8 vs UTF-16 position encoding from client capabilities and converts offsets both ways; `handlers/` contains editor-agnostic logic operating on byte offsets (`definition_at_offset`, `diagnostics`, `document_symbols`, `handlers/completion.rs`'s `completion_at_offset`).
- **`adoc-render`** — `Renderer` trait with `SystemAsciidoctor` (spawns the `asciidoctor` executable with argument vectors, supports unsaved source via stdin, safe modes, attributes, stylesheets) and `MockRenderer` for tests.

Request flow: `didOpen`/`didChange` → `ServerState::update` parses and replaces the document in the index → handlers resolve against the index first, then the Antora catalog → `protocol.rs` maps `SourceRange` to LSP ranges using the negotiated encoding and publishes diagnostics.

Definition resolution order in `handlers/definition.rs`: reference under cursor → Antora resource ID (if the file sits in an Antora module) → plain relative path/anchor; then includes, same order.

### Zed-side assets

`extension.toml` registers the language, snippets (`snippets/asciidoc.json`), the `adoc-ls` language server, and two Tree-sitter grammars pinned by revision. `languages/asciidoc/` and `languages/asciidoc-inline/` hold config plus `.scm` queries (highlights, outline, injections, brackets, indents, overrides). Tree-sitter is for syntax, outline, folding, and injections only — never for attribute evaluation, include expansion, Antora coordinates, or cross-file references.

## Conventions that must be preserved

- Keep the Zed extension thin; all semantics live in the standalone crates.
- Never leak `lsp-types`/transport types into `adoc-core`, `adoc-parser`, `adoc-index`, or `adoc-antora`.
- Diagnostics stay conservative: prefer no diagnostic over a false positive, warnings over errors for unresolved references, and no diagnostics at all for dynamic constructs (`include::{attr}/x.adoc[]`).
- Incomplete AsciiDoc is normal input — parsers and handlers must not panic on half-typed syntax.
- `unsafe_code = "forbid"` workspace-wide; avoid global mutable state.
- All external dependencies are pinned exactly (`=1.0.229` style); keep new ones pinned the same way. No Node.js, no JVM, no database, no custom HTML rendering engine.
- Use `Path`/`PathBuf` for filesystem work and the crate's `normalize_path`; do not canonicalise paths that may not exist.
- Do not clone remote Antora repositories or execute project scripts.
- `crates/zed-asciidoc/` is an empty leftover directory outside the workspace; the extension crate is the repository root (`src/lib.rs`). Do not add code there.
- Do not start future-phase features (rename, references, code actions, cross-component Antora, preview UI) unless asked.

## Testing

Unit tests live in `#[cfg(test)] mod tests` next to the code; `crates/adoc-ls/tests/stdio.rs` drives the real binary through an initialise/shutdown lifecycle via `CARGO_BIN_EXE_adoc-ls`. Shared fixtures are in `tests/fixtures/` grouped by scenario (`simple/`, `includes/`, `xrefs/`, `antora-single-component/`) and are reached from tests with `Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/...")` — never absolute developer-machine paths. Keep tests deterministic and fixtures small; add tests alongside every parser, resolver, renderer, or LSP behaviour change.

Tests in `adoc-render` that spawn a process must take the crate-private `spawn_guard()` mutex in `asciidoctor.rs`. Cargo's test binary is still open for writing while sibling threads fork, so a concurrent `execve` intermittently fails with `ETXTBSY`. Serialising the spawns is the fix — do not remove the guard or add unguarded spawning tests.
