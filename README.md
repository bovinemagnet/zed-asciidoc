# AsciiDoc for Zed

AsciiDoc for Zed is an early-stage Zed extension and language-service project for first-class AsciiDoc, Asciidoctor, and Antora authoring.

The repository currently contains the initial Rust workspace, Zed language metadata, and foundational crates described in the PRDs under `docs/prd/`.

## Current Status

Implemented:

- Cargo workspace bootstrap.
- Thin registered Zed Wasm extension entry point.
- Zed language registration for `.adoc`, `.asciidoc`, and `.ad`.
- Pinned block and inline Tree-sitter grammars with highlighting, outline, and source-block injection queries.
- Core AsciiDoc domain types.
- Minimal semantic parser for titles, sections, attributes, anchors, xrefs, and includes.
- Replacement-based workspace index and basic file/anchor definition resolution.
- Initial Antora model and renderer abstraction, including a system Asciidoctor adapter.
- `adoc-ls` document state, symbols, diagnostics, and definition handler foundations.
- Small deterministic fixtures.

Not implemented yet:

- LSP JSON-RPC transport and Zed language-server adapter.
- Antora descriptor parsing, catalog discovery, and resource resolution.
- Workspace diagnostics for missing xrefs and includes.
- Preview UI and renderer-to-editor integration.

## Development

Run the baseline checks before completing a change:

```sh
cargo check --workspace
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
cargo run -p adoc-ls -- --version
```

Zed dev extension builds require Rust installed through `rustup` with the `wasm32-wasip2` target. Install the repository root as a dev extension to exercise language detection and Tree-sitter queries. The `adoc-ls --stdio` transport is intentionally disabled until protocol handling is implemented.

Use `docs/prd/PRD-AsciiDoc_for_Zed_Initial_Implementation_Specification.md` as the implementation source of truth.
