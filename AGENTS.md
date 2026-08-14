# Repository Guidelines

## Project Structure & Module Organization

This repository is a Rust workspace for an AsciiDoc language extension, semantic engine, and language server.

- `docs/prd/` contains the product requirements and initial implementation specification.
- `crates/` separates core types, parsing, indexing, Antora, rendering, LSP, and Zed integration.
- `languages/` and `extension.toml` define Zed language metadata and Tree-sitter queries.
- `snippets/` contains editor snippets; `tests/fixtures/` contains shared AsciiDoc and Antora examples.

When adding implementation files, keep the Zed extension thin and place semantic behavior in standalone Rust crates such as `adoc-core`, `adoc-parser`, `adoc-index`, `adoc-antora`, `adoc-render`, and `adoc-ls`.

## Build, Test, and Development Commands

- `cargo check --workspace` to verify all crates compile.
- `cargo fmt --check` to enforce Rust formatting.
- `cargo clippy --workspace --all-targets -- -D warnings` to enforce lint-clean code.
- `cargo test --workspace` to run unit and integration tests.
- `cargo check -p asciidoc-zed-extension --target wasm32-wasip2` to verify the Zed Wasm target.
- `cargo run -p adoc-ls -- --version` to smoke-test the standalone binary.

## Coding Style & Naming Conventions

Use Markdown headings and concise, actionable prose in documentation. Keep examples in fenced code blocks when they represent commands, paths, or AsciiDoc snippets.

For Rust code, follow standard `rustfmt` output. Use kebab-case crate names (`adoc-ls`, `adoc-core`) and snake_case modules/functions. Keep LSP transport types out of core domain crates, and avoid adding Node.js tooling for language-server implementation.

## Testing Guidelines

Add tests with each parser, resolver, renderer, or LSP behavior. Prefer deterministic unit and integration tests that run outside Zed. Store small readable fixtures in `tests/fixtures/`, grouped by scenario, for example `simple/`, `includes/`, `xrefs/`, and Antora component layouts.

Do not rely on absolute developer-machine paths or external executables unless the test explicitly verifies missing-executable behavior.

## Commit & Pull Request Guidelines

Current history uses short initial commits, so no detailed convention is established yet. Use concise imperative commit messages, for example `Add initial parser fixtures`.

Pull requests should include a summary, linked issue or PRD section where relevant, tests run, and screenshots only for Zed UI or rendering changes. Keep PRs incremental; each step should leave the workspace compiling once code exists.
