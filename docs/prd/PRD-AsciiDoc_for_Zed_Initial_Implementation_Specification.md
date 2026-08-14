# PRD: AsciiDoc for Zed — Copilot Initial Implementation Specification

## 1. Document Purpose

This document defines the initial implementation plan for **AsciiDoc for Zed**, a Zed editor extension that provides first-class AsciiDoc editing and establishes the foundation for IntelliJ AsciiDoc Plugin feature parity, including Antora-aware authoring.

This specification is intentionally written for implementation by an AI coding agent such as GitHub Copilot. It therefore favors concrete repository layout, responsibilities, interfaces, acceptance criteria, implementation order, test cases, and explicit scope boundaries over high-level product language.

The implementation must be incremental. The first objective is not to implement every IntelliJ feature. The first objective is to establish an architecture that can support those features without requiring a rewrite.

The initial release must provide:

- AsciiDoc file recognition in Zed.
- Tree-sitter based syntax highlighting.
- Document outline support.
- Embedded source-language highlighting where Zed supports language injection.
- A Zed extension written in Rust.
- A standalone Rust language server.
- Basic document symbols and navigation.
- Basic diagnostics.
- Initial Antora workspace discovery.
- Initial Antora page/resource identification.
- A rendering abstraction.
- A first HTML rendering command or external preview fallback.
- Tests and fixtures that can grow into a compatibility suite.

The long-term product goal is a Zed-based AsciiDoc and Antora authoring experience comparable to the IntelliJ AsciiDoc Plugin.

The initial implementation should prioritize correctness of architectural boundaries over raw feature count.

---

## 2. Product Name

Marketplace name:

**AsciiDoc for Zed**

Repository name:

`zed-asciidoc`

Suggested extension identifier:

`asciidoc`

Suggested language server package:

`adoc-ls`

Suggested project codename:

`Zedoc`

Do not expose the codename as the primary marketplace title. Marketplace discoverability is more important than branding.

---

## 3. Product Vision

A user should be able to open an `.adoc` file in Zed and immediately receive syntax-aware editing behavior similar to other first-class Zed languages.

For an ordinary AsciiDoc file:

```asciidoc
= Platform Architecture

== Context

The application uses xref:deployment.adoc[Deployment] for production rollout.

include::partials/security.adoc[]

[source,java]
----
public record User(String name) {}
----
```

the editor should eventually provide:

- accurate syntax highlighting;
- section outline;
- folding;
- highlighting of embedded Java;
- completion for `xref:` targets;
- completion for `include::` targets;
- navigation to referenced files and anchors;
- diagnostics for unresolved resources;
- rendered preview;
- synchronized editor and preview;
- project-level reference indexing.

For an Antora project:

```text
docs/
├── antora.yml
└── modules/
    ├── ROOT/
    │   ├── nav.adoc
    │   ├── pages/
    │   │   └── index.adoc
    │   ├── partials/
    │   ├── examples/
    │   ├── images/
    │   └── attachments/
    └── architecture/
        ├── pages/
        ├── partials/
        ├── examples/
        ├── images/
        └── nav.adoc
```

the extension must understand that a file under:

```text
modules/architecture/pages/overview.adoc
```

is an Antora page with:

- a component derived from `antora.yml`;
- a module named `architecture`;
- a resource family of `page`;
- a relative resource path of `overview.adoc`.

Antora support is not optional or a later bolt-on.

The core workspace model must be designed from the beginning so Antora coordinates can coexist with ordinary filesystem references.

---

## 4. Architectural Principles

The implementation must follow these principles.

### 4.1 Tree-sitter is for editor syntax, not full semantics

Tree-sitter should provide:

- syntax highlighting;
- outline extraction;
- folding structure;
- bracket/structural behavior;
- language injections;
- fast incremental parsing.

Tree-sitter must not become the sole source of truth for:

- attribute evaluation;
- conditional directives;
- include expansion;
- Antora resource coordinates;
- workspace-wide references;
- refactoring;
- preview rendering.

AsciiDoc semantics are context-dependent enough that forcing all behavior into Tree-sitter queries will create an unmaintainable design.

### 4.2 The language server owns semantic intelligence

A standalone Rust LSP named `adoc-ls` should own:

- workspace indexing;
- document metadata;
- symbols;
- references;
- xref resolution;
- include resolution;
- diagnostics;
- completion;
- Antora catalogs;
- hover;
- rename/refactoring in future phases.

The Zed extension should remain relatively thin.

### 4.3 Rendering is an independent subsystem

Rendering should not be conflated with parsing.

Use an abstraction such as:

```rust
pub trait AsciiDocRenderer {
    fn render(&self, request: RenderRequest) -> anyhow::Result<RenderResult>;
}
```

The first implementation may call an external `asciidoctor` executable or use another compatible approach.

Do not implement a complete AsciiDoc renderer in Phase 1.

### 4.4 Antora is a first-class workspace model

The semantic layer must understand:

- component;
- version;
- module;
- family;
- resource path.

Even if Phase 1 only resolves local components, the data model must support multiple components and versions without redesign.

### 4.5 Every feature should be testable outside Zed where practical

Parsing, indexing, Antora resolution, diagnostics, and rendering should all have unit or integration tests that do not require launching Zed.

The Zed extension layer should be the thinnest part of the system.

### 4.6 Prefer conservative semantic analysis

AsciiDoc supports dynamic attributes, includes, conditionals, extensions, and external configuration.

If the language server cannot confidently prove that something is invalid, it should avoid producing an error.

False-positive diagnostics are worse than missing diagnostics for dynamic constructs.

---

## 5. Platform Constraint: Preview

The desired product includes a rendered AsciiDoc preview in a Zed pane or tab, preferably with an **Open Preview to the Side** workflow similar to Markdown.

At the time this PRD is written, the public Zed extension API does not provide a general-purpose arbitrary webview/preview-pane API equivalent to VS Code webviews.

Therefore the implementation must not block Phase 1 on native in-editor preview.

The architecture must support two paths.

### Current extension path

- edit AsciiDoc in Zed;
- render to HTML;
- open an external browser or produce a preview artifact;
- keep renderer interfaces independent of Zed.

### Future native-preview path

- adopt a Zed preview API if one becomes available;
- or contribute a narrowly scoped preview-provider capability upstream;
- connect the existing rendering subsystem to the native preview pane.

Do not embed preview-specific assumptions into the LSP protocol unless they are generic enough to support any editor.

Do not build an embedded browser engine as part of this project.

The lack of a native preview API is a platform constraint, not a reason to compromise the language-server architecture.

---

## 6. Initial Delivery Scope

The initial work is divided into five milestones.

### Milestone A — Repository and Zed Language Skeleton

Deliver:

- Cargo workspace.
- Zed extension crate.
- AsciiDoc language registration.
- `.adoc`, `.asciidoc`, and `.ad` file recognition.
- Tree-sitter grammar integration.
- Initial syntax highlighting.
- Initial outline query.
- CI.
- Basic test fixtures.

### Milestone B — Standalone Language Server

Deliver:

- `adoc-ls` executable.
- `initialize`.
- `textDocument/didOpen`.
- `textDocument/didChange`.
- `textDocument/didClose`.
- `textDocument/documentSymbol`.
- Basic diagnostics.
- Document parsing abstraction.
- Workspace file registry.

### Milestone C — Navigation and Workspace Index

Deliver:

- anchor indexing;
- file indexing;
- basic xref parsing;
- basic include parsing;
- Go to Definition for same-document anchors;
- Go to Definition for local `xref:file.adoc[]`;
- Go to Definition for local `include::file.adoc[]`;
- unresolved-file diagnostics.

### Milestone D — Antora Foundation

Deliver:

- discover `antora.yml`;
- parse component `name`, `version`, `title`, `start_page`, and `nav`;
- discover `modules/`;
- identify module name;
- identify resource family from directory;
- build local Antora resource catalog;
- resolve basic same-component resource IDs;
- expose Antora information through internal APIs;
- basic Antora diagnostics.

### Milestone E — Renderer Foundation

Deliver:

- renderer trait;
- system Asciidoctor implementation;
- renderer discovery;
- render current file to HTML;
- optional external-browser command;
- renderer test fixtures;
- clear error when Asciidoctor is unavailable.

Milestones should be implemented in this order unless a small prerequisite requires otherwise.

Do not begin advanced IntelliJ parity features until these milestones work end to end.

---

## 7. Repository Layout

Create a Cargo workspace with a structure similar to:

```text
zed-asciidoc/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── README.md
├── LICENSE
├── .gitignore
├── .editorconfig
├── extension.toml
│
├── languages/
│   └── asciidoc/
│       ├── config.toml
│       ├── highlights.scm
│       ├── outline.scm
│       ├── injections.scm
│       ├── brackets.scm
│       ├── indents.scm
│       └── overrides.scm
│
├── snippets/
│   └── asciidoc.json
│
├── crates/
│   ├── zed-asciidoc/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   │
│   ├── adoc-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── document.rs
│   │       ├── symbol.rs
│   │       ├── reference.rs
│   │       └── diagnostic.rs
│   │
│   ├── adoc-parser/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── parser.rs
│   │       └── line_parser.rs
│   │
│   ├── adoc-index/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── workspace.rs
│   │       └── index.rs
│   │
│   ├── adoc-antora/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── component.rs
│   │       ├── module.rs
│   │       ├── resource.rs
│   │       └── catalog.rs
│   │
│   ├── adoc-render/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── request.rs
│   │       ├── result.rs
│   │       └── asciidoctor.rs
│   │
│   └── adoc-ls/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── server.rs
│           ├── state.rs
│           ├── capabilities.rs
│           └── handlers/
│               ├── mod.rs
│               ├── document_symbols.rs
│               ├── diagnostics.rs
│               └── definition.rs
│
└── tests/
    └── fixtures/
        ├── simple/
        ├── includes/
        ├── xrefs/
        ├── antora-single-component/
        └── antora-multi-module/
```

Avoid creating dozens of additional crates immediately.

The important requirement is maintaining logical boundaries between:

- Zed integration;
- parser;
- core semantic model;
- workspace index;
- Antora model;
- renderer;
- LSP transport.

Some of these may temporarily begin as modules if doing so significantly accelerates the bootstrap.

---

## 8. Tree-sitter Requirements

Evaluate an existing Tree-sitter AsciiDoc grammar before writing a new grammar.

Do not automatically create a bespoke grammar unless the existing grammar proves unsuitable.

The grammar must support enough structure for the following editor behaviors.

### 8.1 Required syntax captures

Highlight:

- document title;
- section titles;
- block titles;
- attribute declarations;
- attribute references;
- anchors and IDs;
- cross references;
- links;
- include macros;
- image macros;
- admonitions;
- lists;
- description lists;
- comments;
- quoted text;
- emphasis;
- strong emphasis;
- monospace;
- source block attributes;
- block delimiters;
- tables;
- literal blocks;
- passthrough blocks where identifiable.

The first highlighting implementation does not need every obscure AsciiDoc syntax form.

Common Asciidoctor syntax must be prioritized.

### 8.2 Outline

The initial `outline.scm` must expose headings.

For:

```asciidoc
= Product Guide

== Introduction

=== Installation

== Administration
```

the outline should conceptually represent:

```text
Product Guide
├── Introduction
│   └── Installation
└── Administration
```

If nested hierarchy cannot be represented directly by Zed's outline query, expose headings in source order with their names and levels.

### 8.3 Language injection

For:

```asciidoc
[source,java]
----
record User(String name) {}
----
```

the content between delimiters should be injected as Java when supported by Zed.

Minimum language names to test:

- `java`;
- `javascript`;
- `typescript`;
- `json`;
- `yaml`;
- `xml`;
- `html`;
- `css`;
- `sql`;
- `bash`;
- `shell`;
- `python`;
- `ruby`;
- `rust`;
- `go`;
- `c`;
- `cpp`;
- `csharp`;
- `graphql`;
- `toml`.

Do not fail if an injected language is not installed.

In that situation, retain ordinary source-block highlighting.

Language aliases should eventually be configurable, but this is not required for the first milestone.

### 8.4 Tree-sitter fixture tests

Create representative fixture documents and verify captures where practical.

At minimum include:

- headings;
- attributes;
- source blocks;
- nested blocks;
- lists;
- xrefs;
- includes;
- images;
- tables.

Do not attempt to make Tree-sitter evaluate attributes or includes.

---

## 9. Core Document Model

Create editor-independent semantic types in `adoc-core`.

Suggested structures:

```rust
pub struct Document {
    pub uri: Url,
    pub text: String,
    pub title: Option<DocumentTitle>,
    pub sections: Vec<Section>,
    pub attributes: Vec<AttributeDeclaration>,
    pub anchors: Vec<Anchor>,
    pub references: Vec<Reference>,
    pub includes: Vec<IncludeDirective>,
}
```

Use byte offsets or another consistent internal source-range abstraction.

Suggested common range type:

```rust
pub struct SourceRange {
    pub start: usize,
    pub end: usize,
}
```

When converting to LSP ranges, maintain a line index for each document.

Suggested semantic structures:

```rust
pub struct Section {
    pub level: u8,
    pub title: String,
    pub id: Option<String>,
    pub range: SourceRange,
    pub selection_range: SourceRange,
}

pub struct Anchor {
    pub id: String,
    pub range: SourceRange,
}

pub struct AttributeDeclaration {
    pub name: String,
    pub value: Option<String>,
    pub range: SourceRange,
}
```

References should distinguish their meaning.

For example:

```rust
pub enum ReferenceKind {
    LocalAnchor,
    Xref,
    Link,
    Include,
    Image,
    Attribute,
}
```

Generic semantic crates must not expose raw Tree-sitter nodes or LSP-specific structures as their public domain model.

That coupling would make future parser replacement and alternative editor integrations unnecessarily difficult.

---

## 10. Initial Parser Scope

The initial semantic parser is not required to implement the complete AsciiDoc grammar.

It must reliably identify:

- document title;
- section headings;
- explicit anchors;
- attribute declarations;
- basic `xref:` macros;
- shorthand `<<anchor>>` references;
- basic `include::` directives;
- basic `image::` directives;
- useful source ranges.

It should ignore content inside obvious verbatim/source blocks when parsing constructs that should not be interpreted there.

For example:

```asciidoc
[source]
----
xref:not-a-real-reference.adoc[]
----
```

must not initially create a semantic xref if the parser can clearly determine that the content is inside a source/verbatim block.

A line-oriented state machine is acceptable for the initial semantic parser.

Do not create one giant regular expression that attempts to parse all AsciiDoc.

The parser API must allow replacement or extension later.

Suggested interface:

```rust
pub trait DocumentParser: Send + Sync {
    fn parse(
        &self,
        uri: &Url,
        text: &str,
    ) -> ParseResult;
}
```

`ParseResult` should contain:

- parsed `Document`;
- recoverable parser diagnostics where useful;
- sufficient metadata for indexing.

The parser must be tolerant.

Incomplete input is expected while the user types.

A malformed block, unfinished macro, partial attribute declaration, or incomplete heading must not crash the parser or language server.

---

## 11. LSP Server Requirements

Implement `adoc-ls` as a standalone executable.

### 11.1 Minimum capabilities

Initially advertise:

- text document synchronization;
- document symbols;
- Go to Definition;
- diagnostics.

Use incremental text synchronization if implementation is straightforward.

Full-document synchronization is acceptable for the first working version if it substantially simplifies the bootstrap, provided the architecture does not depend permanently on it.

Completion may be enabled in the following milestone.

Design state management so completion can be added without redesign.

### 11.2 Server state

Suggested shape:

```rust
pub struct ServerState {
    pub workspace_roots: Vec<PathBuf>,
    pub documents: DocumentStore,
    pub index: WorkspaceIndex,
    pub antora: AntoraCatalog,
}
```

The exact concurrent containers are implementation details.

Avoid unnecessary locking complexity.

### 11.3 `didOpen`

When a document opens:

1. store its current editor text;
2. parse the document;
3. create/update its semantic representation;
4. replace its workspace index contribution;
5. determine Antora identity if applicable;
6. run diagnostics;
7. publish diagnostics.

### 11.4 `didChange`

When a document changes:

1. apply the change;
2. reparse that document;
3. replace that document's index contribution;
4. recalculate diagnostics for that document;
5. update dependent diagnostics only where necessary.

Do not rescan and reparse the entire workspace for every keystroke.

### 11.5 `didClose`

When a document closes:

- remove the in-memory editor overlay;
- if the file exists on disk, restore disk-backed state;
- if it does not exist, remove it from the index.

Unsaved and newly created files should not cause panics or invalid filesystem assumptions.

---

## 12. Workspace Index

Create a workspace index independent of the LSP transport layer.

Suggested model:

```rust
pub struct WorkspaceIndex {
    files: HashMap<PathBuf, FileEntry>,
    anchors: HashMap<AnchorKey, Vec<AnchorLocation>>,
    references: Vec<ReferenceLocation>,
}
```

`FileEntry` should include:

- normalized path;
- URI;
- parsed metadata;
- content hash or modification metadata;
- Antora identity when applicable.

### 12.1 Initial indexing strategy

At workspace initialization:

- recursively scan workspace roots;
- include `.adoc`;
- include `.asciidoc`;
- include `.ad`;
- ignore common generated/build directories;
- parse each candidate;
- populate file and anchor indexes.

Default ignored directories should include:

```text
.git
.zed
node_modules
build
target
dist
.idea
.gradle
```

Make ignores configurable later.

### 12.2 Index replacement

Each document must own an identifiable contribution to the workspace index.

Updating one file should conceptually perform:

```text
remove previous contribution for file
        ↓
parse changed file
        ↓
insert replacement contribution
```

Do not mutate global reference arrays in a way that leaves stale entries.

### 12.3 Performance rule

Do not design lookup operations around scanning every parsed document.

Use direct maps for common identities.

Examples:

```text
file path -> FileEntry
(document, anchor) -> AnchorLocation
Antora coordinate -> AntoraResource
```

Large repository support is a future optimization task, but the basic data structures must not make it impossible.

---

## 13. Xref Parsing and Navigation

Initial supported forms:

```asciidoc
xref:other.adoc[]
xref:other.adoc[Other Page]
xref:other.adoc#section-id[]
xref:#local-id[]
<<local-id>>
```

Antora resource IDs are handled through the Antora subsystem.

### 13.1 Definition rules

For:

```asciidoc
xref:#local-id[]
```

search the current document's anchor index.

For:

```asciidoc
<<local-id>>
```

search the current document.

For:

```asciidoc
xref:other.adoc[]
```

resolve the path relative to the current file for ordinary AsciiDoc mode.

If the file exists, navigate to:

1. its document title if available;
2. otherwise its first meaningful source position;
3. otherwise line 1.

For:

```asciidoc
xref:other.adoc#security[]
```

resolve the file and then the target anchor.

Do not search arbitrary workspace directories when normal relative resolution fails.

That could produce surprising navigation to unrelated documents sharing the same name.

### 13.2 Diagnostics

Report definitely unresolved local xrefs as warnings.

Example:

```text
Unresolved AsciiDoc xref target: missing.adoc
```

Report unresolved anchors independently.

Example:

```text
Unresolved AsciiDoc anchor: security
```

Avoid duplicate diagnostics for the same source range.

---

## 14. Include Parsing and Navigation

Initial supported syntax:

```asciidoc
include::file.adoc[]
include::partials/security.adoc[]
include::example.java[]
```

Path resolution in ordinary AsciiDoc mode:

1. resolve relative to the current file's directory;
2. apply simple attribute substitution only when attributes are statically known;
3. test whether the resulting target exists;
4. navigate if resolvable;
5. otherwise produce a conservative warning.

Example:

```asciidoc
include::{partialsdir}/security.adoc[]
```

If `{partialsdir}` cannot currently be resolved, do not emit a hard missing-file error.

The target is dynamic rather than definitely invalid.

Go to Definition on a resolvable include should navigate to the included file.

Do not implement full Asciidoctor include semantics in Phase 1.

Support for:

- tags;
- lines;
- optional include attributes;
- URI includes;
- conditional includes;

can come later.

---

## 15. Diagnostics Engine

Create diagnostics using editor-independent domain types rather than constructing LSP diagnostics throughout the parser.

Suggested model:

```rust
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub range: SourceRange,
}
```

Initial diagnostic codes should include:

```text
adoc.unresolved-xref-file
adoc.unresolved-anchor
adoc.unresolved-include
adoc.duplicate-anchor
adoc.antora.unknown-module
adoc.antora.unknown-resource
```

### 15.1 Duplicate anchors

Given:

```asciidoc
[[security]]
== Security

[[security]]
== Other Security
```

report the duplicate on the second declaration.

The first anchor remains the primary definition unless future Asciidoctor compatibility behavior requires something else.

### 15.2 Conservative behavior

Prefer no diagnostic to a false positive when resolution depends on unsupported dynamic behavior.

Examples:

```asciidoc
include::{generated-path}/example.adoc[]
```

or references inside conditionally enabled blocks may not be statically provable.

Diagnostics should therefore distinguish:

- definitely invalid;
- unresolved using current workspace knowledge;
- dynamic/unknown.

In the first version, simply avoiding diagnostics for dynamic constructs is acceptable.

Warnings are preferred over errors for reference-resolution problems.

---

## 16. Antora Detection

A directory is an Antora component root when it contains:

```text
antora.yml
modules/
```

Do not require a `ROOT` module.

The discovery algorithm should:

1. scan workspace roots for `antora.yml`;
2. parse each descriptor;
3. verify a sibling `modules` directory;
4. register the component root;
5. scan modules beneath it.

Nested repositories/components must be allowed.

### 16.1 `antora.yml` model

Initial representation:

```rust
pub struct AntoraComponentDescriptor {
    pub root: PathBuf,
    pub name: String,
    pub title: Option<String>,
    pub version: Option<String>,
    pub display_version: Option<String>,
    pub start_page: Option<String>,
    pub nav: Vec<String>,
    pub asciidoc_attributes: HashMap<String, String>,
}
```

Do not fail an entire component because an optional field is absent.

If `name` is missing, register an appropriate component/descriptor diagnostic.

### 16.2 YAML parsing

Use a maintained YAML library such as `serde_yaml` or its appropriate successor.

Preserve the ability to surface useful parse failures.

The project does not need to implement a YAML language server.

Zed's normal YAML capabilities should remain responsible for generic YAML editing.

The AsciiDoc extension only needs semantic interpretation of Antora-specific fields.

---

## 17. Antora Module and Family Discovery

Under:

```text
modules/<module-name>/
```

recognize resource families:

- `pages`;
- `partials`;
- `examples`;
- `images`;
- `attachments`.

A module may also contain:

```text
nav.adoc
```

Suggested model:

```rust
pub struct AntoraModule {
    pub component: ComponentId,
    pub name: String,
    pub root: PathBuf,
}
```

Resource family:

```rust
pub enum ResourceFamily {
    Page,
    Partial,
    Example,
    Image,
    Attachment,
}
```

Resource:

```rust
pub struct AntoraResource {
    pub component: ComponentId,
    pub version: Option<String>,
    pub module: String,
    pub family: ResourceFamily,
    pub relative_path: PathBuf,
    pub file_path: PathBuf,
}
```

Treat `ROOT` as a legitimate module.

Do not represent `ROOT` as an empty string throughout the internal model merely because some Antora syntax allows shorthand references.

Keep canonical identity distinct from syntax shorthand.

---

## 18. Antora Resource IDs

The resolver must eventually support Antora coordinates such as:

```text
page.adoc
module:page.adoc
component:module:page.adoc
version@component:module:page.adoc
```

and family-qualified resources such as:

```text
partial$intro.adoc
example$sample.java
image$diagram.svg
attachment$guide.pdf
```

Initial Antora support may be restricted to:

- local workspace resources;
- same component;
- same version;
- current or explicitly named module.

Create a real resource-ID parser now.

Suggested type:

```rust
pub struct AntoraResourceId {
    pub version: Option<String>,
    pub component: Option<String>,
    pub module: Option<String>,
    pub family: Option<ResourceFamily>,
    pub path: String,
}
```

Parsing and resolution are separate responsibilities.

Suggested APIs:

```rust
pub fn parse_resource_id(
    input: &str,
) -> Result<AntoraResourceId, ResourceIdParseError>;
```

and:

```rust
pub trait AntoraResolver {
    fn resolve(
        &self,
        id: &AntoraResourceId,
        context: &AntoraContext,
    ) -> ResolutionResult;
}
```

Do not embed filesystem access directly in the resource-ID parser.

A resource ID can be syntactically valid even if its target is unavailable.

---

## 19. Initial Antora Resolution Rules

Support at least:

```asciidoc
xref:index.adoc[]
xref:security:authentication.adoc[]
include::partial$intro.adoc[]
include::example$sample.json[]
image::image$architecture.svg[]
```

Initial interpretation:

### Unqualified page xref

```asciidoc
xref:index.adoc[]
```

Resolve using:

- current component;
- current module;
- page family.

### Module-qualified page xref

```asciidoc
xref:security:authentication.adoc[]
```

Resolve using:

- current component;
- `security` module;
- page family.

### Partial include

```asciidoc
include::partial$intro.adoc[]
```

Resolve using:

- current component;
- current module;
- partial family.

### Example include

```asciidoc
include::example$sample.json[]
```

Resolve using:

- current component;
- current module;
- example family.

### Image

```asciidoc
image::image$architecture.svg[]
```

Resolve using:

- current component;
- current module;
- image family.

If detailed Antora shorthand semantics require correction later, all such logic must be isolated in `adoc-antora`.

Do not scatter Antora-specific path logic across LSP handlers.

---

## 20. Renderer Interface

Create `adoc-render` even if native Zed preview is not yet possible.

Suggested types:

```rust
pub struct RenderRequest {
    pub source_file: PathBuf,
    pub source_text: Option<String>,
    pub attributes: HashMap<String, String>,
    pub safe_mode: RenderSafeMode,
    pub stylesheet: Option<PathBuf>,
}
```

```rust
pub struct RenderResult {
    pub html: String,
    pub warnings: Vec<RenderWarning>,
}
```

Safe-mode representation:

```rust
pub enum RenderSafeMode {
    Secure,
    Safe,
    Server,
    Unsafe,
}
```

The exact translation to Asciidoctor CLI parameters belongs in the renderer implementation.

### 20.1 System Asciidoctor renderer

Initial renderer discovery order:

1. explicitly configured path, once configuration support exists;
2. `asciidoctor` executable available on `PATH`.

Invoke the executable directly.

Do not run Asciidoctor through:

```text
sh -c
bash -c
cmd /c
```

unless an unavoidable platform constraint is proven.

Use direct argument arrays.

Capture:

- stdout;
- stderr;
- exit status.

### 20.2 Unsaved editor content

The long-term preview must work for unsaved content.

The renderer API therefore supports:

```rust
source_text: Option<String>
```

The first implementation may write unsaved content to a temporary file when necessary.

Do not overwrite the user's actual source file.

Preserve source-directory semantics for includes where possible.

### 20.3 Missing Asciidoctor

Return a structured error.

For example:

```rust
pub enum RenderError {
    AsciidoctorNotFound,
    ProcessFailed {
        code: Option<i32>,
        stderr: String,
    },
    Io(std::io::Error),
}
```

The user-facing message should clearly explain:

```text
AsciiDoc preview currently requires an Asciidoctor executable.
```

Do not silently return empty HTML.

### 20.4 Future source maps

Do not implement source mapping yet, but avoid an API that makes it impossible.

A future render result may contain:

```rust
pub struct RenderedSourceLocation {
    pub source_file: PathBuf,
    pub source_range: SourceRange,
    pub rendered_id: String,
}
```

This will later support:

- editor-to-preview scrolling;
- preview-to-editor navigation;
- included partial navigation.

---

## 21. Non-Goals for Initial Work

Do not implement the following until the initial architecture is stable:

- native Zed preview pane where platform APIs are unavailable;
- two-way preview/source synchronization;
- PDF export;
- DOCX export;
- PlantUML;
- Mermaid;
- Graphviz;
- Kroki;
- automatic image paste;
- Markdown-to-AsciiDoc conversion;
- formatted clipboard conversion;
- extract-to-partial refactoring;
- inline-include refactoring;
- workspace-wide rename;
- complete conditional evaluation;
- complete Asciidoctor extension compatibility;
- arbitrary Ruby extension execution;
- remote Git repository cloning from Antora playbooks;
- Antora UI bundle rendering;
- Antora Collector integration;
- complex table editing UI;
- spell-checking implementation;
- custom preview CSS configuration UI;
- remote site publishing;
- Antora site generation.

These are valid future features.

They must not distract the first development cycle from producing a stable language foundation.

Do not add speculative abstractions for every future feature.

Introduce abstractions only where clear subsystem separation is already required.

---

## 22. Implementation Sequence for Copilot

Copilot should execute the work in the following order.

Do not create the entire project in one enormous change.

Each step should leave the workspace compiling.

### Step 1 — Create Cargo workspace

Create:

- root `Cargo.toml`;
- initial crates;
- formatting configuration;
- README;
- license;
- CI skeleton.

Ensure:

```text
cargo check --workspace
```

passes.

### Step 2 — Bootstrap Zed extension

Implement:

- `extension.toml`;
- AsciiDoc language registration;
- file suffixes;
- language configuration.

Confirm `.adoc` can be recognized by Zed.

### Step 3 — Integrate Tree-sitter grammar

Select and integrate an existing AsciiDoc Tree-sitter grammar where practical.

Verify the grammar compiles and loads.

Do not modify the grammar unnecessarily during the initial integration.

### Step 4 — Implement Tree-sitter queries

Create:

```text
highlights.scm
outline.scm
injections.scm
```

Then add optional:

```text
brackets.scm
indents.scm
overrides.scm
```

Start with common syntax.

### Step 5 — Create `adoc-core`

Implement:

- `SourceRange`;
- `Document`;
- `Section`;
- `Anchor`;
- `Reference`;
- `IncludeDirective`;
- diagnostics types.

No Zed or LSP dependencies in this crate.

### Step 6 — Implement minimal semantic parser

Parse:

- document title;
- headings;
- anchors;
- attributes;
- xrefs;
- includes.

Write unit tests immediately.

### Step 7 — Implement workspace index

Index:

- files;
- anchors;
- parsed document metadata.

Create deterministic fixture tests.

### Step 8 — Bootstrap `adoc-ls`

Implement:

- initialize;
- shutdown;
- document synchronization;
- document state.

Ensure the executable can start independently of Zed.

### Step 9 — Implement document symbols

Return document title and headings.

Test using LSP integration tests.

### Step 10 — Implement anchor navigation

Support:

```asciidoc
<<anchor>>
```

and:

```asciidoc
xref:#anchor[]
```

### Step 11 — Implement file xref navigation

Support:

```asciidoc
xref:other.adoc[]
```

and:

```asciidoc
xref:other.adoc#anchor[]
```

### Step 12 — Implement include navigation

Support:

```asciidoc
include::partials/example.adoc[]
```

### Step 13 — Add diagnostics

Implement:

- unresolved local xref;
- unresolved local anchor;
- unresolved local include;
- duplicate anchor.

### Step 14 — Connect Zed to `adoc-ls`

Confirm:

- language server launches;
- document symbols appear;
- diagnostics appear;
- Go to Definition works.

### Step 15 — Parse `antora.yml`

Implement descriptor domain types and YAML parser.

Create Antora fixture.

### Step 16 — Build Antora catalog

Discover:

- components;
- modules;
- pages;
- partials;
- examples;
- images;
- attachments.

### Step 17 — Implement Antora resource-ID parser

Test syntax independently from filesystem resolution.

### Step 18 — Implement Antora resolution

Start with:

- same component;
- same version;
- current module;
- explicitly named module.

### Step 19 — Connect Antora resolver to navigation

Support:

```asciidoc
include::partial$foo.adoc[]
xref:security:authentication.adoc[]
```

### Step 20 — Add renderer abstraction

Implement mock renderer tests before external process handling.

### Step 21 — Add system Asciidoctor renderer

Test successful render where Asciidoctor is available.

Test structured missing-executable behavior everywhere.

### Step 22 — Documentation and cleanup

Update README.

Run:

```text
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

Do not begin Phase 2 until this baseline passes.

---

## 23. Definition of Done for Initial Work

The initial implementation is complete when all of the following are true.

### Repository

- Cargo workspace builds.
- CI runs formatting, Clippy, and tests.
- README explains current status and limitations.

### Zed language support

- extension can be loaded in Zed;
- `.adoc` is recognized;
- `.asciidoc` is recognized;
- `.ad` is recognized;
- common syntax is highlighted;
- headings appear in the outline;
- source blocks support at least Java and JSON injection.

### Language server

- standalone `adoc-ls` executable launches;
- documents can open/change/close;
- document symbols work;
- malformed documents do not crash the server.

### Navigation

- same-document anchors navigate;
- local filesystem xrefs navigate;
- local includes navigate;
- xrefs with anchors navigate.

### Diagnostics

- unresolved local xrefs produce warnings;
- unresolved local anchors produce warnings;
- unresolved local includes produce warnings;
- duplicate anchors produce warnings;
- dynamic unresolved paths do not produce obvious false positives.

### Workspace index

- workspace files are indexed;
- anchors are indexed;
- changing one document replaces that document's contribution;
- changing one document does not require reparsing every workspace file.

### Antora

- `antora.yml` is discovered;
- component name/version can be read;
- modules are discovered;
- resource families are identified;
- an Antora document receives a component/module/family identity;
- same-component Antora pages can resolve;
- same-component partials can resolve.

### Rendering

- renderer interface exists;
- system Asciidoctor implementation exists;
- rendering a simple file produces HTML when Asciidoctor is available;
- missing Asciidoctor returns a useful typed error.

### Tests

Automated tests cover all major initial behaviors.

This is the threshold for moving into completion, advanced Antora support, and native/live preview work.

---

## 24. Critical Design Decisions to Preserve

Copilot must not simplify away the following architectural boundaries.

### Decision 1 — Keep the Zed extension thin

Do not put all parsing, indexing, and Antora logic inside:

```text
zed-asciidoc
```

The Zed crate is an adapter.

### Decision 2 — Do not use Tree-sitter as the semantic engine

Tree-sitter supplies fast syntax structure.

Semantic intelligence belongs in the parser/index/LSP subsystem.

### Decision 3 — Keep Antora resolution separate

Do not implement Antora references as a collection of special cases inside generic filesystem navigation.

Use:

```text
parse resource ID
        ↓
create Antora resolution context
        ↓
resolve through AntoraCatalog
```

### Decision 4 — Keep rendering editor-independent

Rendering must work without Zed.

The same renderer should later power:

- external preview;
- native Zed preview;
- export;
- renderer tests.

### Decision 5 — Do not build a browser engine

Native preview should use Zed platform support when possible.

Until then, external HTML is the fallback.

### Decision 6 — Keep diagnostics conservative

Do not claim a dynamic AsciiDoc construct is broken simply because the initial language server cannot evaluate it.

### Decision 7 — Index incrementally

Do not rebuild the entire workspace after each edit.

### Decision 8 — Keep core types editor-independent

`adoc-core` must not depend on:

- Zed SDK;
- LSP transport;
- browser APIs;
- external command execution.

---

## 25. Copilot Working Rules

When generating code from this PRD, follow these rules.

1. Prefer compiling, tested increments over large speculative changes.

2. Add tests with each parser or resolver feature.

3. Keep platform-specific code in the Zed adapter.

4. Keep LSP transport concerns out of semantic crates.

5. Use typed domain models for Antora resources.

6. Treat incomplete AsciiDoc as normal editor input.

7. Never panic because the user is halfway through typing syntax.

8. Avoid global mutable state.

9. Do not invoke shell command strings for rendering.

10. Use `Path` and `PathBuf` for filesystem paths.

11. Normalize filesystem paths carefully and consistently.

12. Do not require canonicalization of non-existent paths for unsaved-document workflows.

13. Keep diagnostics conservative.

14. Include enough context in logs to diagnose failures.

15. Do not log complete private document contents by default.

16. Run `cargo fmt` before completing a work item.

17. Run `cargo clippy` before completing a work item.

18. Run relevant tests before completing a work item.

19. Update README when setup or visible behavior changes.

20. Explicitly mark unsupported features rather than pretending they work.

21. Do not begin future-phase features because they appear easy.

22. Prefer simple Rust over complex generic frameworks.

23. Keep public interfaces intentionally small.

24. Avoid exposing parser-internal AST nodes outside the parser crate.

25. Avoid exposing LSP-specific types inside core domain crates.

26. Use typed errors where callers need to distinguish failure categories.

27. Preserve deterministic tests.

28. Avoid tests that depend on absolute developer-machine paths.

29. Make test fixtures small and readable.

30. Keep the repository buildable after every major implementation step.

31. Add TODOs only when they refer to clearly defined future work.

32. Do not leave large commented-out experimental implementations.

33. Document architectural deviations from this PRD in code or an ADR.

34. Do not add a database until profiling demonstrates a requirement.

35. Do not add Node.js merely to implement the language server.

36. Do not add a JVM dependency.

37. Do not implement a custom HTML/browser rendering engine.

38. Do not automatically clone remote Antora repositories.

39. Do not silently execute arbitrary project scripts.

40. Optimize for a maintainable path to full IntelliJ AsciiDoc parity.

---

## 26. First Development Target

Use the following miniature repository as the first meaningful end-to-end fixture:

```text
demo/
├── index.adoc
├── architecture.adoc
└── docs/
    ├── antora.yml
    └── modules/
        ├── ROOT/
        │   ├── pages/
        │   │   └── index.adoc
        │   └── partials/
        │       └── welcome.adoc
        └── security/
            ├── pages/
            │   └── authentication.adoc
            └── partials/
                └── token-note.adoc
```

`index.adoc`:

```asciidoc
= Demo

See xref:architecture.adoc[Architecture].
```

`architecture.adoc`:

```asciidoc
= Architecture

[[overview]]
== Overview

[source,java]
----
record Service(String name) {}
----
```

`docs/antora.yml`:

```yaml
name: demo
title: Demo Documentation
version: latest

nav:
  - modules/ROOT/nav.adoc
```

`modules/ROOT/pages/index.adoc`:

```asciidoc
= Documentation Home

xref:security:authentication.adoc[Authentication]
```

`modules/security/pages/authentication.adoc`:

```asciidoc
= Authentication

include::partial$token-note.adoc[]
```

`modules/security/partials/token-note.adoc`:

```asciidoc
NOTE: Authentication tokens must be protected.
```

The finished initial implementation must demonstrate the following.

### Ordinary AsciiDoc

Open:

```text
index.adoc
```

Expected:

- recognized as AsciiDoc;
- title highlighted;
- xref highlighted;
- outline contains `Demo`;
- Go to Definition on `architecture.adoc` opens the target.

Open:

```text
architecture.adoc
```

Expected:

- outline contains `Architecture` and `Overview`;
- `[[overview]]` is recognized;
- Java source block receives Java highlighting.

Adding:

```asciidoc
See <<overview>>.
```

must allow Go to Definition to reach the anchor.

Adding:

```asciidoc
xref:missing.adoc[]
```

must produce a diagnostic.

Removing the reference must clear the diagnostic after the document update.

### Antora

Open:

```text
docs/modules/security/pages/authentication.adoc
```

Expected semantic identity:

```text
component = demo
version = latest
module = security
family = page
path = authentication.adoc
```

For:

```asciidoc
include::partial$token-note.adoc[]
```

Go to Definition must resolve:

```text
docs/modules/security/partials/token-note.adoc
```

For:

```asciidoc
xref:ROOT:index.adoc[]
```

or the canonical supported Antora equivalent, navigation must resolve:

```text
docs/modules/ROOT/pages/index.adoc
```

A deliberately missing local resource should produce a conservative Antora diagnostic.

### Rendering

When Asciidoctor is installed:

```text
index.adoc
```

must render to non-empty HTML containing the document title.

If Asciidoctor is not installed, the renderer should return a structured, user-understandable error instead of crashing.

---

# Appendix A — Expected Future Architecture

The eventual dependency structure should remain approximately:

```text
                           Zed
                            │
                  ┌─────────┴─────────┐
                  │                   │
            Tree-sitter           Zed Extension
                                      │
                                      ▼
                                   adoc-ls
                                      │
                 ┌────────────────────┼────────────────────┐
                 │                    │                    │
                 ▼                    ▼                    ▼
             adoc-parser          adoc-index          adoc-antora
                 │                    │                    │
                 └────────────────────┼────────────────────┘
                                      │
                                      ▼
                                  adoc-core

                              adoc-render
                                   │
                                   ▼
                              Asciidoctor
```

The exact crate graph may differ slightly, but dependencies must point inward toward reusable semantic components.

In particular:

```text
adoc-core
```

must not depend on:

```text
Zed
tower-lsp
Asciidoctor process APIs
preview APIs
```

---

# Appendix B — Phase 2 After Initial Acceptance

Once all initial Definition of Done criteria pass, begin Phase 2.

Priority order:

1. `xref:` completion.
2. `include::` completion.
3. Antora page completion.
4. Antora partial/example/image completion.
5. attribute completion.
6. hover.
7. find references.
8. workspace symbols.
9. richer Antora diagnostics.
10. `antora.yml` semantic completion.
11. `nav.adoc` intelligence.
12. multi-component Antora workspace support.
13. live/external preview refresh.
14. native preview integration when Zed platform support permits it.

Completion examples:

```asciidoc
xref:sec
```

should eventually suggest matching pages/modules.

```asciidoc
include::partial$
```

should suggest partials in the current Antora module.

```asciidoc
include::partial$tok
```

should rank:

```text
token-note.adoc
```

highly.

Future page attributes such as:

```asciidoc
{page-
```

should expose appropriate Antora attributes when their values are known.

---

# Appendix C — Longer-Term IntelliJ Parity

The following remain product requirements, but they are not requirements for the first implementation cycle:

- live rendered preview;
- Open Preview to Side;
- editor-to-preview synchronization;
- preview-to-editor navigation;
- included-source navigation;
- configurable preview stylesheet;
- diagrams;
- PlantUML;
- Mermaid;
- Graphviz;
- Kroki;
- HTML export;
- PDF export;
- DOCX export where practical;
- Markdown-to-AsciiDoc conversion;
- image paste handling;
- formatted clipboard conversion;
- find all references;
- rename anchor;
- page rename/update references;
- extract include;
- inline include;
- table helpers;
- Asciidoctor extension configuration;
- `.asciidoctorconfig`;
- advanced attributes;
- conditional directives;
- Antora playbooks;
- multiple Antora components;
- multiple Antora component versions;
- Antora `nav.adoc`;
- Antora start-page support;
- deeper Antora playbook attributes;
- workspace-scale performance optimization.

Full parity should be measured as workflow parity rather than identical IntelliJ UI.

---

# Appendix D — Guiding Engineering Rule

The most important rule for the implementation is:

> **Zed is the editor client; `adoc-ls` is the documentation intelligence platform.**

That means the majority of the product's valuable intelligence should remain usable independently of Zed.

The same semantic engine should theoretically be reusable by another LSP-capable editor without rewriting:

- AsciiDoc parsing;
- anchor indexing;
- xref resolution;
- diagnostics;
- Antora cataloging;
- Antora resource resolution;
- completion;
- refactoring.

The renderer should similarly remain independent.

This avoids creating a large Zed-specific extension that becomes difficult to test and maintain.

---

# Appendix E — Completion Criteria for Copilot's First Coding Cycle

Do not consider the first coding cycle complete because the repository exists or syntax highlighting works.

The first coding cycle should end with a vertical slice across the major architectural layers:

```text
.adoc document
     │
     ├── Tree-sitter → highlighting + outline
     │
     └── adoc-ls
            │
            ├── parser
            │      ↓
            │   semantic document
            │
            ├── workspace index
            │      ↓
            │   xref/include navigation
            │
            ├── diagnostics
            │
            └── Antora catalog
                   ↓
              resource navigation
```

Rendering should additionally prove:

```text
AsciiDoc source
      ↓
adoc-render
      ↓
Asciidoctor
      ↓
HTML
```

At that point the project has a defensible foundation.

Only after this foundation is passing automated tests should development move toward richer completion, live preview, diagrams, refactoring, and full IntelliJ parity.

The goal of this PRD is therefore not merely to produce an AsciiDoc syntax extension.

The goal is to establish the initial architecture for a **first-class AsciiDoc and Antora development environment in Zed**.