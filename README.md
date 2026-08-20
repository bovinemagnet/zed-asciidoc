# AsciiDoc for Zed

AsciiDoc for Zed is an early-stage Zed extension and language-service project for first-class AsciiDoc, Asciidoctor, and Antora authoring.

The repository currently contains the initial Rust workspace, Zed language metadata, and foundational crates described in the PRDs under `docs/prd/`.

## Current Status

Implemented:

- Cargo workspace bootstrap.
- Thin registered Zed Wasm extension entry point.
- Zed language registration for `.adoc`, `.asciidoc`, and `.ad`.
- Pinned block and inline Tree-sitter grammars with highlighting, outline, and source-block injection queries.
  The grammars come from a fork carrying a `table_header_row` node, which the upstream grammar does not
  expose, so a table's header row can be styled apart from its body.
- Core AsciiDoc domain types.
- Minimal semantic parser for titles, sections, attributes, anchors, xrefs, and includes.
- Replacement-based workspace index and basic file/anchor definition resolution.
- Renderer abstraction and direct system Asciidoctor adapter for file or unsaved source, safe modes, attributes, and custom stylesheets.
- `adoc-ls` stdio transport with incremental synchronization, document symbols, diagnostics, and Go to Definition.
- Workspace indexing and conservative diagnostics for missing local xrefs, anchors, and includes.
- Reference resolution that follows Asciidoctor and Antora semantics: module-root-relative page IDs,
  implicit and natural section references, anchors declared in included partials, and bibliography anchors.
- Antora descriptor parsing plus deterministic component, module, and resource-family discovery.
- Same-component Antora xref/include navigation and unknown module/resource diagnostics; components
  absent from the workspace are assumed to come from elsewhere in the playbook and are not reported.
- Completion for `xref:` targets, `include::` targets, `image:` targets, and anchors.
  Inside an Antora module the current module's pages are offered as bare IDs and other
  modules' pages module-qualified, `include::` offers the family prefixes and then that
  family's resources, and anchors come from the file a target names. In a plain workspace
  the same constructs complete relative paths, including non-AsciiDoc include targets.
- Zed language-server registration using an `adoc-ls` executable available on `PATH`.
- HTML preview via two code actions. `AsciiDoc: render preview` renders once;
  `AsciiDoc: render live preview` additionally re-renders whenever the document is saved
  and embeds a reloader so the page keeps up; while a document is being followed that
  action becomes `AsciiDoc: stop live preview`, and closing the buffer stops it too. Both render the
  open buffer (saved or not) with Asciidoctor,
  merges Antora page and family-directory attributes (`moduledir`, `pagesdir`,
  `partialsdir`, `examplesdir`, `attachmentsdir`, `imagesdir`), rewrites
  family-qualified includes such as
  `partial$note.adoc` to absolute paths,
  and opens the result in the default browser. Includes nested inside an
  included file are rewritten too, via copies in a scratch directory; reading those
  copies is why preview renders with Asciidoctor's `unsafe` safe mode. Cyclic includes
  stop at the repeat rather than recursing.
- Small deterministic fixtures.

Not implemented yet:

- Cross-component/version Antora selection and `antora.yml` editor diagnostics.
- Rename and references. The only code action is the preview command above.
- A preview pane inside Zed. The extension API exposes no webview or preview capability,
  so preview opens externally; see
  `docs/prd/DESIGN-Preview_Pipeline_and_Native_Rendering.md`.
- A pure-Rust renderer. Preview currently requires the `asciidoctor` executable on `PATH`.

## Development

Run the baseline checks before completing a change:

```sh
cargo check --workspace
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
cargo run -p adoc-ls -- --version
```

Install the language-server executable before loading the repository root as a Zed dev extension:

```sh
cargo install --path crates/adoc-ls
```

Zed dev extension builds require Rust installed through `rustup` with the `wasm32-wasip2` target. Once loaded, the extension starts `adoc-ls --stdio` from `PATH` for AsciiDoc buffers.

Use `docs/prd/PRD-AsciiDoc_for_Zed_Initial_Implementation_Specification.md` as the implementation source of truth.
