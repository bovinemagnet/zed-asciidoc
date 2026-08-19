# Design: Preview Pipeline and Native Rendering

| | |
|---|---|
| Author | Paul Snow |
| Version | 0.0.0 |
| Date | 2026-08-18 |
| Status | Proposed |
| Supersedes | Nothing. Expands PRD §5 (Platform Constraint: Preview) and §4.3 (Rendering is an independent subsystem). |

## 1. Purpose

This note designs two separable pieces of work:

- **Part A** — the plumbing that turns `adoc-render` from an unused subsystem into a
  user-visible preview, delivered through the language server rather than the extension.
- **Part B** — the path to removing the Asciidoctor (Ruby) runtime dependency by adopting
  a pure-Rust renderer behind the existing `Renderer` trait.

Part A is buildable now and does not depend on Part B. Part B is gated on third-party
maturity and must not block anything.

## 2. Platform findings

These were verified against Zed `main` and the pinned `zed_extension_api = "=0.7.0"`, and
they constrain everything below.

- **A Zed extension cannot present a preview pane.** The `Extension` trait exposes
  language servers, slash commands, context servers, docs indexing, debug adapters, and
  completion/symbol labels. There is no webview, preview, or arbitrary-command hook.
- **The extension cannot launch a windowed process.** `zed_extension_api::process` exposes
  a single method, `Command::output()`, which blocks and captures. There is no detached
  spawn, and no user-triggered entry point that would call it.
- **Zed's Markdown preview is not an extension.** `crates/markdown_preview` is a
  first-party crate compiled into the binary, built directly on `gpui`, and hardcoded to
  Markdown (`language.name() == "Markdown"`). `zed_actions::preview` contains exactly two
  submodules, `markdown` and `svg`, both in-tree. There is a preview namespace but no
  provider interface.
- **Zed does have an HTML-to-gpui renderer**, in `crates/markdown/src/html/`
  (`html_parser.rs`, `html_rendering.rs`, `html_minifier.rs`). It handles paragraphs,
  images, lists, tables, and styled text. It is `pub(crate)` and scoped to HTML embedded
  within Markdown, with no CSS engine and no JavaScript. It is precedent, not a public API.
- **`gpui` is published**: v0.2.2, Apache-2.0. A standalone gpui application is feasible.
- **Zed honours `workspace/executeCommand`.** `crates/project/src/lsp_store.rs` issues
  `lsp::request::ExecuteCommand` and gates on the server's `execute_command_provider`
  capability.

**Conclusion.** The language server, not the extension, must own the preview. A native
in-editor pane is unreachable today; the design must therefore isolate *delivery* so the
final step can be replaced without disturbing anything above it.

## 3. Current state

`adoc-render` is complete and tested but wired to nothing:

```rust
pub trait Renderer: Send + Sync {
    fn render(&self, request: &RenderRequest) -> Result<RenderOutput, RenderError>;
}
```

`SystemAsciidoctor` spawns the `asciidoctor` executable and already supports unsaved
source via stdin, safe modes, attributes, and custom stylesheets. `MockRenderer` exists
for tests. `RenderRequest::from_source` covers the unsaved-buffer case.

`adoc-ls` advertises `definitionProvider`, `documentSymbolProvider`, `textDocumentSync`
and `positionEncoding`. It has no code-action or execute-command support.

## 4. Part A — Preview plumbing

### 4.1 Chain

```
user invokes code action in Zed
      |  textDocument/codeAction
adoc-ls returns CodeAction { command: "adoc.renderPreview", args: [uri] }
      |  workspace/executeCommand
adoc-ls execute_command handler
      |
RenderRequest -> Renderer -> RenderOutput { html, warnings }
      |
PreviewSink                     <-- the only piece that changes later
      |
browser today  ·  gpui window tomorrow
```

### 4.2 Capabilities

Added in `crates/adoc-ls/src/protocol.rs`:

```rust
code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
execute_command_provider: Some(ExecuteCommandOptions {
    commands: vec!["adoc.renderPreview".to_owned()],
    ..Default::default()
}),
```

### 4.3 New modules

| File | Responsibility |
|---|---|
| `crates/adoc-ls/src/handlers/code_actions.rs` | offer the preview action for AsciiDoc buffers |
| `crates/adoc-ls/src/handlers/execute_command.rs` | build the request, render, hand off to a sink |
| `crates/adoc-ls/src/preview.rs` | `PreviewSink` trait and `BrowserSink` |

Handlers stay editor-agnostic and return plain data; `protocol.rs` performs the mapping to
`lsp_types`. This preserves the rule that no crate below `adoc-ls` sees LSP types, and
that handlers operate on byte offsets rather than positions.

### 4.4 Code action

```rust
pub struct PreviewAction {
    pub title: String,
    pub uri: String,
}

pub fn code_actions_for(uri: &str) -> Vec<PreviewAction> {
    vec![PreviewAction {
        title: "AsciiDoc: Render preview".to_owned(),
        uri: uri.to_owned(),
    }]
}
```

### 4.5 Execute

```rust
pub fn render_preview(
    state: &ServerState,
    renderer: &dyn Renderer,
    sink: &dyn PreviewSink,
    uri: &str,
) -> Result<PathBuf, PreviewError> {
    let document = state.document(uri).ok_or(PreviewError::NotOpen)?;

    // Unsaved buffers are already supported: from_source feeds the renderer via stdin.
    let mut request = RenderRequest::from_source(document.path(), document.text());

    // adoc-ls knows the Antora coordinates; a bare Asciidoctor invocation never would.
    request.attributes.extend(state.antora_attributes_for(uri));

    let output = renderer.render(&request)?;
    sink.deliver(&output, document.path())
}
```

### 4.6 The swap point

```rust
pub trait PreviewSink: Send + Sync {
    fn deliver(&self, output: &RenderOutput, source: &Path) -> Result<PathBuf, PreviewError>;
}

pub struct BrowserSink { dir: PathBuf }   // today
pub struct GpuiSink { socket: PathBuf }   // later
```

`BrowserSink` writes `<dir>/<stem>.html` and opens it with `open` (macOS), `xdg-open`
(Linux), or `cmd /c start` (Windows). A future `GpuiSink` hands the same HTML to a
separate process over a socket.

Delivery lives in `adoc-ls`, not `adoc-render`: the renderer crate stays a pure rendering
abstraction with no opinion about where output goes.

The property that matters: **renderer, LSP command, code action and Antora attribute
handling are all unchanged by a sink swap.** If a native preview capability ever lands
upstream, only `PreviewSink` is rewritten.

### 4.7 Testing

- `code_actions_for` returns the action for an open AsciiDoc document — unit test.
- `render_preview` with `MockRenderer` and a recording sink asserts that unsaved text
  reaches the request and that Antora attributes are merged — unit test, no process spawn.
- `BrowserSink` writes the expected file and returns its path; the launch step is behind a
  seam so tests never open a browser.
- Any test that spawns a real process must take the `spawn_guard()` mutex in
  `adoc-render/src/asciidoctor.rs`, per the existing `ETXTBSY` workaround.

### 4.8 Estimate

Approximately one day. `adoc-render` is done; this is two handlers, a small trait, one
implementation, and the capability wiring.

## 5. Part B — Removing the Ruby requirement

### 5.1 Why the abstraction already wins

A pure-Rust renderer is a third implementation of `Renderer`, beside `SystemAsciidoctor`
and `MockRenderer`. Nothing above it changes. PRD §4.3 bought this.

### 5.2 Prior art

The `asciidoc-rs` organisation is building a matched parser and renderer pair.

| Crate | Version | Downloads | Updated | Assessment |
|---|---|---|---|---|
| `asciidoc-parser` | 0.29.19 | 16,537 | 2026-08-09 | author describes it as largely feature-complete, with very high coverage across 70 releases |
| `asciidoc-html5` | 0.1.7 | 440 | 2026-08-13 | author describes it as in its infancy; targets output compatible with Asciidoctor's `html5` backend |
| `asciidocr` | 0.1.14 | 8,731 | 2025-12-18 | pure-Rust CLI and library, but eight months without a release |
| `acdc-parser` | 0.9.0 | 581 | 2026-04-26 | PEG-based alternative parser |

`asciidoc-html5` is validated against both the AsciiDoc language description and
Asciidoctor's reference test suite, with spec-coverage tracking.

Declared exclusions from its 1.0 scope, all relevant to us:

- HTML5 only — no DocBook, man page, or XHTML backends.
- Client-side highlighters only (`highlight.js`, `prettify`); CodeRay, Pygments and Rouge
  are not planned.
- No Asciidoctor extension API.
- `include::` encodings limited to UTF-8, ISO-8859-1 and windows-1252.

### 5.3 Adoption

```rust
pub struct NativeRenderer;

impl Renderer for NativeRenderer {
    fn render(&self, request: &RenderRequest) -> Result<RenderOutput, RenderError> {
        let source = request
            .source_text
            .clone()
            .map_or_else(|| fs::read_to_string(&request.source_file), Ok)?;
        let document = asciidoc_parser::Parser::default().parse(&source);
        let html = asciidoc_html5::render(&document /*, attributes */)?;
        Ok(RenderOutput { html, warnings: collect_warnings(&document) })
    }
}
```

Select at runtime rather than compile time: attempt `NativeRenderer`, fall back to
`SystemAsciidoctor`, and expose a setting to force either. That yields a Ruby-free default
with an escape hatch, and allows fidelity to be measured against real documents instead of
estimated.

### 5.4 Antora is an advantage, not a gap

Antora does not use stock Asciidoctor; it drives asciidoctor.js with extensions that
resolve `xref:` page references and Antora attributes. No general Rust renderer will have
those. `adoc-ls` already carries `AntoraCatalog`, `AntoraResolver` and `parse_resource_id`,
so resolving Antora coordinates into concrete paths and attributes *before* handing source
to any renderer is work already done here, and is portable across both renderer backends.

### 5.5 Writing our own renderer is out of scope

AsciiDoc has no formal specification; Asciidoctor is the de facto one, refined over a
decade. The `asciidoc-rs` author is working through the language description against
Asciidoctor's reference tests and expects that discipline to slow progress considerably.
Reproducing it would be a multi-year detour from building an editor extension. If
`asciidoc-html5` blocks us, contributing a fix upstream is far cheaper than a parallel
implementation.

### 5.6 Risk

The Ruby-free path depends on a third-party project maturing on its own timetable, and
`asciidoc-html5` is currently self-described as not meaningfully useful. Nothing may be
sequenced behind it. The mitigation is the runtime fallback in §5.3: ship with
`SystemAsciidoctor`, add `NativeRenderer` behind a setting, and change the default only
when real documents render correctly.

## 6. The gpui window, and upstream absorption

A standalone gpui application that previews AsciiDoc is feasible — `gpui` is published and
Apache-2.0 — and mirroring the structure of `crates/markdown_preview` would make eventual
upstreaming a port rather than a rewrite. This matches the future path already named in
PRD §5.

The blocker for absorption is not the window; it is the runtime dependency. Markdown and
SVG previews are self-contained and render in-process with no external tools. A preview
built on `SystemAsciidoctor` requires a Ruby gem, which upstream will not accept.
**Part B is therefore a precondition for Part C, not an optimisation of it.** A further
obstacle remains even then: Zed's HTML-to-gpui renderer is `pub(crate)`, so it would have
to be reimplemented or made public by agreement.

Sequencing implication: build Part A now, adopt Part B when the ecosystem allows, and only
then evaluate Part C.

## 7. Acceptance criteria

**Part A**

- `adoc-ls` advertises `codeActionProvider` and `executeCommandProvider` with
  `adoc.renderPreview`.
- Invoking the action on an AsciiDoc buffer, including an unsaved one, produces an HTML
  artefact and opens it externally.
- Antora attributes known to the index are present in the render request.
- Handlers contain no `lsp-types` references.
- `cargo test --workspace`, `cargo fmt --check` and
  `cargo clippy --workspace --all-targets -- -D warnings` all pass.

**Part B**

- `NativeRenderer` implements `Renderer` with no process spawn.
- Backend selection is a runtime setting with `SystemAsciidoctor` fallback.
- A fixture corpus renders under both backends, with differences recorded.

## 8. Open questions

- Where should preview artefacts be written — a temporary directory, or alongside the
  source? Temporary avoids polluting worktrees but breaks relative image references.
- Should the preview refresh on `didChange`, or only on explicit invocation? Refresh
  implies debouncing and a persistent sink.
- Does the fallback in §5.3 belong in `adoc-render` as a composite `Renderer`, or in
  `adoc-ls` as selection logic? The former keeps `adoc-ls` thinner; the latter keeps
  `adoc-render` free of policy.
