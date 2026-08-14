# Product Requirements Document: AsciiDoc for Zed

**Working name:** AsciiDoc for Zed  
**Codename:** Zedoc  
**Suggested repository:** `zed-asciidoc`  
**Suggested extension ID:** `asciidoc`  
**Suggested language server:** `zed-asciidoc-ls`  
**Document status:** Draft  
**Target editor:** Zed  
**Primary formats:** AsciiDoc / Asciidoctor / Antora  
**Parity target:** IntelliJ AsciiDoc Plugin

---

# 1. Executive Summary

AsciiDoc for Zed is a Zed extension and supporting language-service ecosystem that provides first-class authoring, navigation, validation and preview support for AsciiDoc documents.

The long-term objective is functional parity, where practical, with the IntelliJ AsciiDoc Plugin, including:

- AsciiDoc syntax highlighting.
- Structural document awareness.
- Live rendered preview.
- Source-to-preview and preview-to-source synchronization.
- Auto-completion.
- Cross-reference navigation.
- Attribute completion.
- Include resolution.
- Diagnostics and inspections.
- Refactoring.
- Embedded source-language highlighting.
- Diagrams.
- HTML/PDF export.
- Asciidoctor configuration.
- Antora-aware content resolution.
- Antora component/module/family awareness.
- Antora resource ID completion and validation.
- Antora navigation support.
- Multi-component workspaces.

The IntelliJ plugin currently provides a context-sensitive editor with syntax highlighting, completion and inline validation, configurable live preview, Antora integration, diagram support, Markdown conversion and HTML/PDF/DOCX generation. These capabilities establish the principal parity baseline for this product.

The implementation should deliberately separate three concerns:

1. **Tree-sitter**
   - Fast syntactic understanding inside Zed.
   - Highlighting.
   - document outline.
   - folding and structural navigation.
   - embedded-language injection.

2. **AsciiDoc Language Server**
   - Semantic understanding.
   - workspace indexing.
   - diagnostics.
   - completion.
   - xref/include resolution.
   - Antora awareness.
   - refactoring.

3. **AsciiDoc Renderer**
   - Authoritative document rendering.
   - Asciidoctor compatibility.
   - diagrams.
   - attributes/includes/extensions.
   - live preview.
   - export.

This separation is important because AsciiDoc is substantially more context-dependent than languages that can be understood solely from a Tree-sitter parse tree.

---

# 2. Important Zed Platform Constraint

There is currently a platform dependency that materially affects this PRD.

As of August 2026, Zed's public extension model officially supports extension-provided languages, debuggers, themes, icon themes, snippets and MCP servers. Language extensions may provide Tree-sitter grammars and language servers.

However, Zed extensions currently do **not** have a general-purpose API for creating arbitrary visual editor tabs, preview panes or webviews.

A Zed issue requesting webview support remains open and explicitly identifies document preview panes as one of the intended use cases.

Zed itself already has the desired UX pattern internally for Markdown, including actions such as:

- Open Preview.
- Open Preview to the Side.
- Open Following Preview.
- Close and Return to Editor.

This should be the UX model for AsciiDoc rather than inventing a separate preview interaction.

Therefore this product should be developed as **two coordinated tracks**:

### Track A — Zed Marketplace Extension

Build everything possible using the existing Zed extension API:

- language registration;
- Tree-sitter grammar;
- syntax highlighting;
- outline;
- injections;
- LSP integration;
- completion;
- diagnostics;
- navigation;
- Antora support;
- snippets;
- formatting;
- external-browser preview fallback.

### Track B — Zed Preview Capability

Implement or contribute a small upstream Zed API allowing extensions to register read-only preview providers.

Preferred capability:

```text
PreviewProvider
    file/language -> rendered document model
```

rather than immediately requesting unrestricted browser/WebView functionality.

Once available:

```text
AsciiDoc editor
        |
        +------ AsciiDoc Preview
                   |
                   +-- open in same pane
                   +-- open to side
                   +-- synchronized navigation
```

A native constrained preview API is preferable to unrestricted WebViews because it preserves Zed's performance, security and UI consistency.

---

# 3. Product Vision

AsciiDoc should feel like a first-class language in Zed rather than a markup format with basic highlighting.

Opening:

```text
architecture.adoc
```

should provide an experience comparable to editing Java, Rust or Markdown:

```text
┌────────────────────────────────┬────────────────────────────────┐
│ architecture.adoc              │ Preview: architecture.adoc     │
│                                │                                │
│ = Architecture                 │ Architecture                   │
│                                │                                │
│ == Context                     │ Context                        │
│                                │                                │
│ xref:deployment.adoc[]         │ Deployment                     │
│                                │                                │
│ include::partial$intro.adoc[]  │ <rendered included content>    │
│                                │                                │
└────────────────────────────────┴────────────────────────────────┘
```

For Antora repositories, the extension should understand:

```text
docs/
├── antora.yml
└── modules/
    ├── ROOT/
    │   ├── pages/
    │   ├── partials/
    │   ├── examples/
    │   ├── images/
    │   ├── attachments/
    │   └── nav.adoc
    └── architecture/
        ├── pages/
        ├── partials/
        ├── examples/
        ├── images/
        └── nav.adoc
```

This matches Antora's standard component structure, where `antora.yml` and `modules` identify a content source root and modules can contain the `pages`, `partials`, `examples`, `images`, and `attachments` resource families.

---

# 4. Goals

## 4.1 Primary Goals

The product shall:

1. Provide high-quality AsciiDoc editing in Zed.
2. Provide rendered preview alongside the editor.
3. Support standard Asciidoctor syntax.
4. Understand project-level includes and references.
5. provide first-class Antora repository support.
6. Approach feature parity with IntelliJ's AsciiDoc Plugin.
7. Maintain Zed's expected low-latency editing experience.
8. Work on macOS, Linux and Windows.
9. Support large documentation repositories.
10. Avoid requiring a JVM.

---

# 5. Non-Goals

Initial releases will not attempt to:

- implement the complete AsciiDoc processor in Tree-sitter;
- replace Antora;
- implement an HTML browser engine;
- implement arbitrary JavaScript execution inside preview;
- automatically clone every remote repository referenced by an Antora playbook;
- reproduce IntelliJ-specific UI concepts exactly;
- support every third-party Asciidoctor extension in the first release.

The goal is **feature-equivalent workflow**, not API-level compatibility with IntelliJ.

---

# 6. Personas

## Documentation Engineer

Maintains a large Antora documentation site consisting of many components and modules.

Requires:

- xref completion;
- includes;
- Antora resource IDs;
- navigation;
- preview;
- validation.

## Software Developer

Maintains AsciiDoc documentation alongside source code.

Requires:

- highlighting;
- preview;
- code blocks;
- links;
- diagrams;
- source-code includes.

## Technical Writer

Uses AsciiDoc as the primary content-authoring environment.

Requires:

- completion;
- spell checking integration;
- structural navigation;
- preview;
- images;
- refactoring;
- export.

## Documentation Architect

Works across multiple repositories and Antora components.

Requires:

- cross-component navigation;
- Antora playbook awareness;
- attributes;
- resource validation;
- repository-scale diagnostics.

---

# 7. High-Level Architecture

```text
                         Zed
                          │
           ┌──────────────┼───────────────┐
           │              │               │
           ▼              ▼               ▼
     Tree-sitter       Zed LSP       Preview Provider
        layer           Client          API
           │              │               │
           │              ▼               │
           │       zed-asciidoc-ls        │
           │              │               │
           │       ┌──────┼──────┐        │
           │       │      │      │        │
           │       ▼      ▼      ▼        │
           │     AST   Antora Workspace   │
           │          Catalog  Index      │
           │              │               │
           └──────────────┼───────────────┘
                          │
                          ▼
                  Rendering Service
                          │
              ┌───────────┴──────────┐
              ▼                      ▼
          Asciidoctor            fallback
          compatible             renderer
              │
              ▼
             HTML
```

---

# 8. Tree-sitter Strategy

A Tree-sitter grammar is required for the Zed language extension.

Zed uses Tree-sitter for language parsing and exposes Tree-sitter queries for syntax highlighting, matching, outline generation, indentation, code injections, syntax overrides and other editor functionality.

An existing open-source `tree-sitter-asciidoc` grammar already provides separate block and inline AsciiDoc grammars and should be evaluated rather than immediately creating a grammar from scratch.

## Required Tree-sitter Queries

The extension should provide:

```text
languages/asciidoc/
├── config.toml
├── highlights.scm
├── outline.scm
├── injections.scm
├── indents.scm
├── brackets.scm
├── textobjects.scm
└── overrides.scm
```

### Highlighting

Capture at least:

- document titles;
- section headings;
- attribute declarations;
- attribute references;
- macros;
- xrefs;
- links;
- include directives;
- admonitions;
- block attributes;
- block titles;
- IDs;
- anchors;
- inline emphasis;
- strong text;
- monospace;
- comments;
- passthrough content;
- lists;
- description lists;
- table delimiters;
- source block delimiters.

### Embedded Language Injection

Source blocks should activate the relevant Zed language grammar.

Example:

```asciidoc
[source,java]
----
public record User(String name) {}
----
```

The Java block should receive Java syntax highlighting.

Required aliases include:

```text
java
kotlin
groovy
javascript
typescript
json
yaml
xml
html
css
sql
bash
sh
shell
powershell
python
ruby
rust
go
c
cpp
csharp
graphql
dockerfile
toml
properties
```

Aliases should be configurable.

---

# 9. Language Server

The semantic layer should be implemented as a standalone Rust language server:

```text
zed-asciidoc-ls
```

Zed extensions can attach one or more LSP servers to a language, making LSP the appropriate boundary for advanced language features.

The language server should maintain a workspace model independent of the Tree-sitter parse tree.

## Core Services

```text
DocumentParser
WorkspaceIndexer
AttributeResolver
IncludeResolver
XrefResolver
AntoraCatalog
DiagnosticsEngine
CompletionEngine
NavigationEngine
RenameEngine
RenderCoordinator
```

---

# 10. Parser Strategy

Do not make the LSP depend directly on Tree-sitter's syntax tree for all semantic behaviour.

Instead:

### Tree-sitter

Optimized for:

```text
editor speed
highlighting
folding
outline
language injection
```

### Semantic parser

Optimized for:

```text
attributes
conditionals
includes
anchors
xref resolution
Antora coordinates
workspace indexing
validation
refactoring
render context
```

An implementation spike should evaluate the Rust **ACDC** project before building another complete semantic parser. ACDC currently contains a Rust AsciiDoc parser, LSP implementation, cross-file anchor indexing, diagnostics, completion, rename, references and semantic-token capabilities.

Decision gate:

```text
IF ACDC passes compatibility tests
    reuse/contribute to ACDC
ELSE
    implement zed-asciidoc-ls parser/index
```

The public extension should not become permanently dependent on an immature parser merely to reduce initial development time.

---

# 11. Rendering Architecture

Rendering correctness should be treated differently from parsing correctness.

For parity with established AsciiDoc tooling, the canonical renderer should be **Asciidoctor-compatible**.

Recommended renderer abstraction:

```rust
trait AsciiDocRenderer {
    fn render(
        document: &Path,
        context: RenderContext
    ) -> RenderResult;
}
```

Implementations:

```text
SystemAsciidoctorRenderer
BundledRenderer
AntoraRenderer
```

## Renderer Selection

Recommended order:

1. Project-configured renderer.
2. Installed `asciidoctor`.
3. Bundled compatible renderer.
4. Error with installation guidance.

This permits users with complex Ruby Asciidoctor extensions to use their actual local processor while still allowing zero-configuration use cases where possible.

---

# 12. Live Preview Requirements

The IntelliJ plugin provides live Asciidoctor-based preview, configurable split orientation, editor-to-preview scrolling, preview-to-editor navigation, local link navigation, custom CSS and zoom.

The Zed extension should target equivalent functionality.

## Commands

```text
AsciiDoc: Open Preview
AsciiDoc: Open Preview to the Side
AsciiDoc: Close Preview
AsciiDoc: Refresh Preview
AsciiDoc: Toggle Preview
AsciiDoc: Export HTML
AsciiDoc: Export PDF
```

## Behaviour

When source changes:

```text
buffer update
     ↓
debounce 150–300 ms
     ↓
render
     ↓
update preview
```

Do not require the document to be saved.

## Scroll Synchronization

Two-way synchronization:

```text
editor cursor
      ↓
nearest AST block
      ↓
preview element
```

and:

```text
preview click
      ↓
source-location metadata
      ↓
file + line
      ↓
editor cursor
```

Included files must be supported.

For example:

```asciidoc
include::partial$security.adoc[]
```

Clicking rendered content originating from `security.adoc` should open that file.

---

# 13. Preview Security

Support:

```text
SECURE
SERVER
SAFE
UNSAFE
```

or equivalent processing modes where supported by the selected rendering engine.

Remote resources should be disabled by default in safe modes.

Potentially dangerous capabilities include:

- remote includes;
- arbitrary scripts;
- user-installed extensions;
- external diagram processors;
- file reads outside the project.

The user should explicitly opt into unsafe behaviour where applicable.

---

# 14. Editor Features

## P0

- `.adoc` recognition.
- `.asciidoc` recognition.
- `.ad` recognition.
- `.asciidoctorconfig` recognition where appropriate.
- syntax highlighting.
- folding.
- document outline.
- auto-closing delimiters.
- source block language injection.
- snippets.

## P1

- attribute completion;
- macro completion;
- filename completion;
- anchor completion;
- xref completion;
- include completion;
- document link navigation;
- hover information.

## P2

- block-aware selections;
- formatting commands;
- table assistance;
- URL paste conversion;
- image paste support;
- Markdown-to-AsciiDoc conversion.

The IntelliJ implementation additionally supports automatic closing of constructs, intelligent include/xref completion, clipboard image insertion, formatted-text conversion and several AsciiDoc-specific editor intentions.

---

# 15. Document Outline

The Zed outline should expose:

```text
Document title
  Section
    Subsection
      Subsection
```

Optionally:

```text
Anchors
Attributes
Includes
```

Example:

```asciidoc
= Platform Architecture

== Context

=== Authentication

[[authorization]]
=== Authorization
```

Outline:

```text
Platform Architecture
├── Context
│   ├── Authentication
│   └── Authorization
```

---

# 16. Auto-Completion

Completion should be context-aware.

## Attribute Completion

Typing:

```asciidoc
{page-
```

might offer:

```text
page-component-name
page-component-version
page-module
page-relative-src-path
page-version
```

## Macro Completion

Typing:

```text
inc
```

offers:

```asciidoc
include::[]
```

Typing:

```text
xref:
```

offers available pages and anchors.

## Block Completion

Typing:

```text
[source,
```

offers installed/supported language names.

---

# 17. Navigation

Implement:

### Go to Definition

For:

```asciidoc
xref:architecture.adoc[]
```

```asciidoc
<<authorization>>
```

```asciidoc
include::partial$intro.adoc[]
```

```asciidoc
image::architecture.svg[]
```

```asciidoc
{my-attribute}
```

### Find References

Support:

- anchors;
- attributes;
- files;
- Antora resource IDs.

### Workspace Symbols

Expose:

- section headings;
- explicit IDs;
- attributes;
- tags.

IntelliJ currently indexes named AsciiDoc elements including headings, IDs, attributes and tags for project-level searching.

---

# 18. Diagnostics

Diagnostics should use standard LSP diagnostics.

Severity:

```text
Error
Warning
Information
Hint
```

## Required Diagnostics

### Broken links

```asciidoc
xref:missing.adoc[]
```

### Unknown anchors

```asciidoc
<<does-not-exist>>
```

### Missing include

```asciidoc
include::missing.adoc[]
```

### Missing image

```asciidoc
image::missing.png[]
```

### Unknown attribute

```asciidoc
{unknown}
```

where validation is appropriate.

### Duplicate anchor

```asciidoc
[[foo]]
...
[[foo]]
```

### Antora errors

```text
unknown component
unknown module
unknown resource
invalid family
invalid coordinate
```

### Suspected Markdown syntax

Examples:

```markdown
# Heading
```

instead of:

```asciidoc
= Heading
```

Quick fix:

```text
Convert to AsciiDoc heading
```

This corresponds to the sort of editor inspection already provided by the IntelliJ plugin.

---

# 19. Quick Fixes

Potential LSP code actions:

```text
Create missing anchor
Create missing page
Create missing partial
Convert Markdown heading
Convert Markdown horizontal rule
Convert Markdown code fence
Insert explicit section ID
Change unresolved xref
Add missing attribute
Suppress diagnostic
```

---

# 20. Refactoring

## Phase 3 Refactorings

### Rename Anchor

Before:

```asciidoc
[[old-name]]

xref:#old-name[]
```

After:

```asciidoc
[[new-name]]

xref:#new-name[]
```

All workspace references change.

### Rename Page

Renaming:

```text
architecture.adoc
```

to:

```text
system-architecture.adoc
```

updates:

```asciidoc
xref:architecture.adoc[]
```

to:

```asciidoc
xref:system-architecture.adoc[]
```

### Extract Include

Select:

```asciidoc
This paragraph contains reusable content.
```

Action:

```text
AsciiDoc: Extract to Partial
```

Result:

```asciidoc
include::partial$reusable-content.adoc[]
```

### Inline Include

Reverse operation.

The IntelliJ plugin supports extracting and inlining include directives, so these belong in the parity roadmap.

---

# 21. Antora Detection

Antora mode should activate when an ancestor contains:

```text
antora.yml
modules/
```

This is also the detection model used by the IntelliJ plugin.

Example:

```text
repo/
└── documentation/
    ├── antora.yml
    └── modules/
```

A page at:

```text
documentation/modules/admin/pages/users.adoc
```

should produce:

```text
component = <antora.yml:name>
module = admin
family = page
resource = users.adoc
```

---

# 22. Antora Resource Model

The language server should model resources using:

```text
version
component
module
family
relative path
```

Internal type:

```rust
struct AntoraResourceId {
    version: Option<String>,
    component: Option<String>,
    module: Option<String>,
    family: ResourceFamily,
    path: String,
}
```

Families:

```rust
enum ResourceFamily {
    Page,
    Partial,
    Example,
    Image,
    Attachment,
}
```

Antora's standard directory model defines these resource families under each module.

---

# 23. Antora Resource-ID Completion

Support:

```asciidoc
xref:
```

and syntax including:

```text
page.adoc
module:page.adoc
component:module:page.adoc
version@component:module:page.adoc
```

Also support family-qualified resources:

```asciidoc
include::partial$intro.adoc[]
include::example$sample.java[]
image::image$architecture.svg[]
xref:page$architecture.adoc[]
```

Completion should filter as coordinates are entered.

Example:

```text
xref:platform:sec<TAB>
```

might offer:

```text
platform:security:index.adoc
platform:security:authentication.adoc
platform:security:authorization.adoc
```

---

# 24. Antora Cross-Component Navigation

The extension must index every Antora component visible within the Zed workspace.

Example:

```text
workspace/
├── platform-docs/
│   └── antora.yml
├── api-docs/
│   └── antora.yml
└── operations-docs/
    └── antora.yml
```

Cross-component reference:

```asciidoc
xref:api:rest:endpoints.adoc[]
```

must support:

- completion;
- validation;
- go-to-definition;
- find references.

The IntelliJ plugin performs comparable component/module resolution by scanning Antora descriptors available in the project.

---

# 25. Antora `antora.yml` Support

Recognize:

```yaml
name:
title:
version:
display_version:
start_page:
nav:
asciidoc:
  attributes:
```

Provide:

- YAML schema validation;
- completion;
- hover documentation.

Additionally provide semantic completion for:

```yaml
start_page:
nav:
```

because these values reference project resources rather than arbitrary strings.

The IntelliJ plugin similarly provides schema-aware editing plus special completion for `nav` and `start_page`.

---

# 26. Antora Attributes

Read:

```yaml
asciidoc:
  attributes:
    product-name: Example
    release-state: beta
```

Expose these while editing:

```asciidoc
{product-name}
```

Also populate Antora intrinsic page attributes where possible.

Examples:

```text
page-component-name
page-component-version
page-module
```

The IntelliJ plugin incorporates attributes from component descriptors and, when available, playbooks into editing and preview context.

---

# 27. Antora Playbook Support

Recognize common playbook names including:

```text
antora-playbook.yml
antora-playbook-local.yml
antora-dev-playbook.yml
```

Parse:

```yaml
content:
  sources:

asciidoc:
  attributes:

ui:

output:
```

## Phase 1

Workspace-local components only.

## Phase 2

Resolve checked-out directories referenced through playbooks.

## Phase 4

Optional deeper Antora catalog integration.

Possible implementation:

```text
antora playbook
      ↓
catalog adapter
      ↓
zed-asciidoc-ls
```

Do not automatically clone arbitrary Git repositories without explicit user permission.

---

# 28. Antora `nav.adoc`

Treat:

```text
nav.adoc
```

as a specialized AsciiDoc file.

Provide:

- page completion;
- xref resolution;
- broken-page diagnostics;
- navigation structure outline.

Example:

```asciidoc
* xref:index.adoc[Introduction]
** xref:architecture.adoc[Architecture]
** xref:deployment.adoc[Deployment]
```

Optional future feature:

```text
AsciiDoc: Show Antora Navigation
```

which displays:

```text
Introduction
├── Architecture
└── Deployment
```

---

# 29. Antora-Aware Preview

When rendering an Antora page, normal standalone Asciidoctor behaviour is insufficient.

The renderer must establish:

```text
imagesdir
partialsdir
examplesdir
attachmentsdir
page-component-name
page-component-version
page-module
```

and resolve Antora resource IDs.

The IntelliJ plugin already performs analogous preview adaptation for images, includes, links, Antora attributes and local Antora references.

---

# 30. Multi-Repository Antora Workspaces

Antora commonly spans multiple Git repositories.

The extension should support a Zed workspace such as:

```text
workspace/
├── antora-site/
├── product-docs/
├── developer-docs/
└── api-docs/
```

The index should treat all workspace roots as a single documentation universe where appropriate.

Phase 4 can optionally read the Antora playbook to distinguish sites when several Antora sites are present in the same workspace.

---

# 31. Diagrams

Parity roadmap should include:

```text
PlantUML
Graphviz
Mermaid
Ditaa
Kroki
```

Example:

```asciidoc
[plantuml]
----
Alice -> Bob: request
Bob --> Alice: response
----
```

Architecture:

```text
AsciiDoc
   ↓
renderer
   ↓
diagram extension/provider
   ↓
SVG
   ↓
preview
```

Prefer SVG in preview.

Diagram rendering should be cached by content hash.

---

# 32. Math

Support common Asciidoctor math constructs such as:

```asciidoc
stem:[sqrt(4) = 2]
```

and block equations.

Rendering should preferably generate MathML or SVG depending on the preview implementation.

---

# 33. Preview Styling

Settings:

```json
{
  "asciidoc.preview.theme": "auto",
  "asciidoc.preview.stylesheet": null,
  "asciidoc.preview.antora_theme": true
}
```

Modes:

```text
Auto
Light
Dark
Custom
Antora
```

When an Antora page is detected, default to an Antora-like stylesheet rather than generic Asciidoctor styling.

---

# 34. Export

Commands:

```text
AsciiDoc: Export HTML
AsciiDoc: Export PDF
AsciiDoc: Export DOCX
```

Priority:

### Phase 3

HTML.

### Phase 4

PDF.

### Phase 5

DOCX.

HTML and PDF are substantially more important than DOCX and should not be delayed while waiting for DOCX support.

---

# 35. Formatting

Implement:

```text
editor: format
```

through the language server.

Options should eventually include:

```text
one sentence per line
blank lines around sections
list indentation
attribute ordering
table formatting
block spacing
```

Respect `.editorconfig` where suitable.

---

# 36. Snippets

Bundle common snippets:

```text
section
source block
example block
sidebar
admonition
table
image
link
xref
include
partial include
example include
PlantUML
Mermaid
open block
quote
listing
literal block
```

Example:

```text
ad-note
```

expands to:

```asciidoc
NOTE: 
```

---

# 37. File Operations

Where Zed's extension APIs eventually allow it, provide commands for:

```text
New Antora Page
New Antora Partial
New Antora Module
New Antora Component
```

Example:

```text
AsciiDoc: New Antora Page
```

creates:

```text
modules/<module>/pages/<name>.adoc
```

with:

```asciidoc
= Page Title
```

---

# 38. Feature Parity Matrix

| Capability | IntelliJ baseline | Zed target |
|---|---:|---:|
| Syntax highlighting | Yes | Phase 1 |
| Live preview | Yes | Phase 2 |
| Editor → preview sync | Yes | Phase 2 |
| Preview → editor sync | Yes | Phase 2 |
| Include navigation | Yes | Phase 2 |
| Xref navigation | Yes | Phase 2 |
| Attribute completion | Yes | Phase 2 |
| File completion | Yes | Phase 2 |
| Named-element search | Yes | Phase 2 |
| Diagnostics | Yes | Phase 2 |
| Quick fixes | Yes | Phase 3 |
| Antora detection | Yes | Phase 2 |
| Antora xrefs | Yes | Phase 2 |
| Antora includes | Yes | Phase 2 |
| Antora completion | Yes | Phase 2 |
| `antora.yml` schema | Yes | Phase 2 |
| Antora attributes | Yes | Phase 2 |
| Multi-component workspace | Yes | Phase 3 |
| Diagrams | Yes | Phase 3 |
| Custom stylesheets | Yes | Phase 3 |
| Markdown → AsciiDoc | Yes | Phase 4 |
| Extract include | Yes | Phase 4 |
| Inline include | Yes | Phase 4 |
| HTML export | Yes | Phase 3 |
| PDF export | Yes | Phase 4 |
| DOCX export | Yes | Phase 5 |
| Paste image | Yes | Phase 5 |
| Formatted paste conversion | Yes | Phase 5 |

---

# 39. Delivery Phases

# Phase 0 — Architecture and Spikes

**Purpose:** Eliminate the high-risk technical uncertainties.

Deliver:

- Tree-sitter evaluation.
- ACDC LSP evaluation.
- Asciidoctor rendering spike.
- Antora catalog prototype.
- Zed preview API investigation.
- external-browser preview fallback.
- cross-platform packaging proof.

Acceptance:

```text
✓ .adoc recognized in Zed
✓ Tree-sitter parses representative documents
✓ LSP starts from Zed extension
✓ test document renders correctly
✓ basic Antora page classified correctly
✓ preview API implementation strategy selected
```

---

# Phase 1 — First-Class Editing

**Release:** `0.1`

Deliver:

- language registration;
- Tree-sitter grammar;
- syntax highlighting;
- source-code injections;
- outline;
- folding;
- auto-closing delimiters;
- snippets;
- basic LSP;
- document symbols;
- local anchor navigation;
- basic completion;
- HTML rendering command;
- external preview fallback.

The release should already be useful even before native preview support exists.

---

# Phase 2 — Preview + Antora MVP

**Release:** `0.2`

This is the first release that fulfills the core product proposition.

Deliver:

### Preview

- Open Preview.
- Open Preview to Side.
- live rendering.
- editor → preview sync.
- preview → editor sync.
- images.
- links.
- includes.
- custom CSS.

### Semantic Editing

- xref completion.
- include completion.
- anchor completion.
- diagnostics.
- hover.
- go-to-definition.

### Antora

- detection.
- component descriptors.
- modules.
- pages.
- partials.
- examples.
- images.
- attachments.
- resource IDs.
- xref validation.
- include resolution.
- component/module completion.
- Antora preview context.

Acceptance scenario:

```asciidoc
xref:security:authentication.adoc[]
```

must:

```text
✓ complete
✓ validate
✓ navigate
✓ render correctly
```

---

# Phase 3 — Professional Authoring

**Release:** `0.3`

Deliver:

- diagrams;
- Antora `nav.adoc` intelligence;
- rename anchor;
- find references;
- workspace symbols;
- custom stylesheet support;
- HTML export;
- Antora multi-component indexing;
- code actions;
- richer diagnostics;
- formatting;
- playbook-local attributes;
- improved preview security.

At this point the extension should be suitable for everyday professional documentation development.

---

# Phase 4 — IntelliJ Parity

**Release:** `0.5`

Deliver:

- Extract Include.
- Inline Include.
- Markdown → AsciiDoc conversion.
- PDF export.
- advanced Antora playbook support.
- conditional rendering awareness.
- `.asciidoctorconfig`.
- Asciidoctor extension configuration.
- advanced formatting.
- remote-resource security controls.
- Antora Collector-aware indexing where practical.

---

# Phase 5 — Extended Ecosystem Parity

**Release:** `1.0`

Deliver remaining high-value parity features:

- DOCX export.
- clipboard image insertion.
- formatted clipboard → AsciiDoc.
- table editing helpers.
- advanced preview interactions.
- comprehensive Asciidoctor extension support.
- additional diagrams.
- large Antora-site optimization.
- full settings UI where supported by Zed.

Version `1.0` means:

> A developer who primarily uses IntelliJ for its AsciiDoc/Antora support can realistically move that documentation workflow to Zed.

---

# 40. Performance Requirements

## Editing

Tree-sitter highlighting:

```text
target < 16 ms incremental parse
```

under typical document edits.

## Completion

Local completion:

```text
target < 50 ms
```

Workspace/Antora completion:

```text
target < 150 ms
```

for a warm index.

## Diagnostics

Do not re-index the entire workspace on every keystroke.

Use:

```text
filesystem watcher
       ↓
incremental re-index
       ↓
affected references
       ↓
diagnostics
```

## Preview

Keystroke-to-preview:

```text
target < 500 ms
```

for ordinary documents.

Use debouncing and cancellation:

```text
Edit 1
Edit 2
Edit 3
   ↓
cancel obsolete renders
   ↓
render Edit 3
```

---

# 41. Large Repository Requirements

Target:

```text
10,000 AsciiDoc files
100,000 anchors
100,000 xrefs
multiple Antora components
multiple workspace roots
```

Index structure:

```text
WorkspaceIndex
├── files
├── anchors
├── attributes
├── includes
├── links
└── AntoraCatalog
    ├── components
    ├── versions
    ├── modules
    └── resources
```

Persist workspace cache under Zed's extension cache where possible.

Cache entries should include:

```text
file
mtime/hash
parsed symbols
references
Antora coordinate
```

---

# 42. Configuration

Example:

```json
{
  "asciidoc": {
    "preview": {
      "enabled": true,
      "live_update": true,
      "theme": "auto",
      "stylesheet": null,
      "safe_mode": "safe"
    },

    "renderer": {
      "engine": "auto",
      "asciidoctor_path": null
    },

    "antora": {
      "enabled": "auto",
      "scan_workspace": true,
      "playbook": null
    },

    "diagrams": {
      "enabled": true,
      "provider": "auto"
    },

    "validation": {
      "links": true,
      "includes": true,
      "anchors": true,
      "antora": true
    }
  }
}
```

---

# 43. Extension Repository Layout

Recommended:

```text
zed-asciidoc/
├── extension.toml
├── Cargo.toml
├── src/
│   └── lib.rs
│
├── languages/
│   └── asciidoc/
│       ├── config.toml
│       ├── highlights.scm
│       ├── outline.scm
│       ├── injections.scm
│       ├── brackets.scm
│       ├── indents.scm
│       ├── overrides.scm
│       └── textobjects.scm
│
├── snippets/
│   └── asciidoc.json
│
├── crates/
│   ├── asciidoc-ls/
│   ├── asciidoc-parser/
│   ├── asciidoc-antora/
│   ├── asciidoc-renderer/
│   └── asciidoc-index/
│
└── tests/
    ├── fixtures/
    ├── antora/
    ├── rendering/
    └── lsp/
```

The components should be separate crates even if initially stored in one repository.

---

# 44. Testing Strategy

A serious compatibility test suite is essential.

## Tree-sitter Tests

Test:

- headings;
- attributes;
- lists;
- tables;
- source blocks;
- macros;
- nested blocks;
- inline formatting;
- conditionals.

## LSP Golden Tests

Input:

```asciidoc
xref:missing.adoc[]
```

Expected:

```text
Diagnostic:
  AsciiDoc unresolved reference
```

## Antora Fixtures

Build test projects including:

```text
single component
multiple modules
multiple components
ROOT module
named modules
nested pages
partials
examples
images
attachments
nav
playbooks
versioned references
```

## Renderer Compatibility

Maintain a corpus:

```text
input.adoc
expected-asciidoctor.html
zed-rendered.html
```

Perform normalized DOM comparison rather than raw HTML string comparison.

## Real-World Repository Tests

Use representative open-source Antora projects as integration fixtures where licensing permits.

---

# 45. Compatibility Philosophy

The extension should distinguish:

```text
syntax compatibility
semantic compatibility
rendering compatibility
```

These are not the same.

For example, Tree-sitter may correctly recognize:

```asciidoc
include::partial$foo.adoc[]
```

without knowing what `partial$` resolves to.

The Antora semantic service resolves it.

The renderer then determines what content appears.

This layering avoids the common mistake of turning one parser into an increasingly complicated source of truth for unrelated concerns.

---

# 46. Observability and Diagnostics

Provide an output/logging mechanism containing:

```text
extension version
language server version
renderer selected
Asciidoctor version
workspace roots
Antora roots
indexed documents
render duration
index duration
```

Commands:

```text
AsciiDoc: Show Environment
AsciiDoc: Restart Language Server
AsciiDoc: Rebuild Workspace Index
AsciiDoc: Diagnose Antora Project
```

Example:

```text
AsciiDoc for Zed

Extension: 0.3.2
Language Server: 0.3.2

Renderer:
  /usr/local/bin/asciidoctor
  version 2.x

Antora:
  Components: 4
  Modules: 17
  Resources: 3,482

Index:
  Documents: 2,731
  Anchors: 9,184
  Xrefs: 14,201
```

This will substantially simplify issue reporting.

---

# 47. Preview API Proposal to Zed

Instead of waiting indefinitely for generic WebViews, propose a narrowly scoped API.

Conceptually:

```rust
trait PreviewProvider {
    fn can_preview(&self, language: LanguageId) -> bool;

    async fn render(
        &self,
        document: DocumentSnapshot,
    ) -> PreviewDocument;
}
```

Possible response:

```rust
struct PreviewDocument {
    content: PreviewContent,
    source_map: SourceMap,
}
```

With:

```rust
enum PreviewContent {
    Html(String),
    NativeDocument(DocumentModel),
}
```

Commands provided by Zed:

```text
preview::Open
preview::OpenToSide
preview::Refresh
preview::Close
```

This could eventually support:

```text
AsciiDoc
HTML
LaTeX
Typst
reStructuredText
Jupyter-like documents
```

rather than implementing another hard-coded viewer for every format.

---

# 48. Major Risks

## Risk: Zed preview API remains unavailable

**Impact:** High.

Mitigation:

```text
Phase 1 external preview
+
upstream PreviewProvider proposal
+
maintain preview renderer independently
```

The LSP/editor work remains valuable regardless.

---

## Risk: Tree-sitter grammar incompleteness

**Impact:** Medium.

Mitigation:

- fork upstream grammar if needed;
- contribute fixes upstream;
- maintain extensive parser fixtures;
- do not use Tree-sitter as the sole semantic parser.

---

## Risk: AsciiDoc semantic complexity

**Impact:** High.

Attributes, includes and conditionals make complete static analysis difficult.

Mitigation:

```text
best-effort editor model
+
authoritative renderer
```

Diagnostics should avoid false positives when resolution is uncertain.

---

## Risk: Antora remote components

**Impact:** Medium.

The editor may not have every branch/version/component checked out.

Mitigation:

Unresolved references to unavailable versions should be treated differently from references that are provably invalid.

For example:

```text
version exists locally
+ component exists locally
+ module missing
    → error

version not present locally
    → unresolved external / informational
```

This follows the same pragmatic approach used by mature Antora IDE tooling.

---

# 49. Success Metrics

Technical:

```text
Crash-free sessions > 99.9%
Completion p95 < 150 ms
Warm navigation p95 < 100 ms
Preview update p95 < 500 ms
```

Quality:

```text
>95% IntelliJ parity test scenarios supported by 1.0
<1% false-positive unresolved xref diagnostics
```

Adoption:

```text
extension installs
weekly active users
Antora workspace usage
preview usage
```

---

# 50. Recommended Initial Engineering Sequence

I recommend implementing the project in this order:

```text
1. Tree-sitter AsciiDoc language
           ↓
2. Basic Rust LSP
           ↓
3. Workspace/index architecture
           ↓
4. Antora resource catalog
           ↓
5. Completion/navigation/diagnostics
           ↓
6. Renderer
           ↓
7. External preview
           ↓
8. Native Zed preview API contribution
           ↓
9. Preview source mapping
           ↓
10. Advanced IntelliJ parity
```

Do **not** start with the preview.

The highest-value foundational piece is the semantic workspace model because completion, validation, navigation, Antora support and preview source mapping all depend on it.

---

# 51. Proposed Internal Crates

I would structure the implementation as:

```text
adoc-syntax
```

Basic AST/types shared across services.

```text
adoc-index
```

Workspace symbols and references.

```text
adoc-antora
```

Antora component/resource catalog.

```text
adoc-render
```

Renderer abstraction.

```text
adoc-ls
```

LSP server.

```text
zed-asciidoc
```

Thin Zed integration.

This avoids putting the entire implementation inside the Zed extension's WASM component.

---

# 52. Architectural Principle

The critical architectural rule should be:

> **Zed is the client; `adoc-ls` is the documentation intelligence platform.**

That provides a useful long-term consequence.

The same language server could later support:

```text
Zed
Neovim
Helix
VS Code
Emacs
Eclipse
```

The bulk of the engineering investment therefore remains editor-independent.

---

# 53. Name Recommendation

## Recommended Marketplace Name

**AsciiDoc for Zed**

Why:

- instantly searchable;
- obvious function;
- matches common extension naming conventions;
- avoids users needing to know a brand name.

Extension identifier:

```text
asciidoc
```

Repository:

```text
zed-asciidoc
```

## Recommended Project Codename

**Zedoc**

Pronounced roughly:

```text
Zed-doc
```

This works well for repository discussions and internal naming but is less useful as the marketplace title.

Other reasonable alternatives:

```text
AsciiDoc + Antora
AsciiDoc Toolkit
Zed AsciiDoc
Adoc for Zed
Zedoc
```

My preference remains:

```text
Marketplace: AsciiDoc for Zed
Repository:  zed-asciidoc
Codename:    Zedoc
LSP:         adoc-ls
```

---

# 54. Definition of Done for 1.0

`1.0` is complete when a user can open a substantial Antora repository in Zed and:

1. Edit `.adoc` files with accurate syntax highlighting.
2. See embedded source-language highlighting.
3. Navigate the document outline.
4. Complete attributes.
5. Complete xrefs.
6. Complete includes.
7. Follow xrefs.
8. Follow includes.
9. Find references to anchors.
10. Rename anchors safely.
11. Receive diagnostics for broken references.
12. Work across Antora components/modules.
13. Resolve pages/partials/examples/images/attachments.
14. Edit `antora.yml` intelligently.
15. Edit `nav.adoc` intelligently.
16. Preview the current page in Zed.
17. See included Antora resources in the preview.
18. Click rendered content to navigate back to its source.
19. Have the preview follow the editor.
20. Render diagrams.
21. Apply custom stylesheets.
22. Export HTML.
23. Export PDF.
24. Use project Asciidoctor configuration.
25. Work in a multi-repository Antora workspace.

At that point AsciiDoc is no longer merely "supported" by Zed.

It is a **first-class technical documentation environment**.

---

# 55. Final Architectural Recommendation

The project should ultimately look like this:

```text
                     ┌─────────────────┐
                     │       Zed       │
                     └───────┬─────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
              ▼              ▼              ▼
        Tree-sitter        LSP Client    Preview UI
              │              │              │
              │              ▼              │
              │          adoc-ls            │
              │              │              │
              │     ┌────────┼────────┐     │
              │     │        │        │     │
              │     ▼        ▼        ▼     │
              │  Parser   Workspace Antora  │
              │           Index     Catalog │
              │     │        │        │     │
              │     └────────┼────────┘     │
              │              │              │
              │              ▼              │
              │          Renderer ──────────┘
              │              │
              │              ▼
              │         Asciidoctor
              │
              ▼
        instant editor
          feedback
```

This architecture gives the project the best chance of reaching IntelliJ-level functionality without creating an unmaintainable monolithic Zed extension.

The two especially important decisions are:

**First:** use Tree-sitter for editing mechanics, not AsciiDoc semantics.

**Second:** make Antora a core semantic model inside the language server rather than bolting Antora path handling onto ordinary AsciiDoc completion later.

Those two choices will determine whether the extension remains a syntax-highlighting plugin or grows into a genuine IntelliJ AsciiDoc replacement.