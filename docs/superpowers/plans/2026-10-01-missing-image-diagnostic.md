# Missing-Image Diagnostic Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Warn `adoc.unresolved-image` for a block `image::` whose target does not exist, without judging files whose images resolve against a document that includes them.

**Architecture:** `adoc-index` gains a reverse include map (`includers`) and a plain-path image resolver, and reports missing images for files nothing includes. `adoc-ls` replaces that answer inside Antora modules with `image$` resource resolution against the page's module, and never judges partials.

**Tech Stack:** Rust workspace (`adoc-core`, `adoc-index`, `adoc-antora`, `adoc-ls`); no new dependencies.

**Spec:** `docs/prd/DESIGN-Missing_Image_Diagnostic.md`

## Global Constraints

- The index answers queries through direct maps; never scan every document per request.
- `adoc-index` must not depend on `adoc-antora`; Antora-aware decisions live in `adoc-ls`.
- Paths are resolved lexically with `normalize_path`; never canonicalise.
- Prefer no diagnostic to a false positive; dynamic targets produce nothing.
- No `lsp-types` outside `adoc-ls`. No new dependencies.
- Always pass `--workspace` or `-p <crate>` to cargo; a bare `cargo test` covers only the extension crate.
- British spelling in comments, messages and docs.

**Deviation from spec §4.5:** `resolve_image` (the hover-facing resolver) has no consumer until hover lands, so it ships with the hover change. This plan creates `handlers/images.rs` with only `antora_image_id`, which both diagnostics here and `resolve_image` later use, so the two still cannot disagree.

---

### Task 1: Includers map in `WorkspaceIndex`

**Files:**
- Modify: `crates/adoc-index/src/index.rs` (struct fields, `replace`, `remove`, new `includers`, tests)

**Interfaces:**
- Consumes: `crate::diagnostics::resolve_include_target(document: &Document, current_path: &Path, target: &str) -> Option<PathBuf>` (exists).
- Produces: `WorkspaceIndex::includers(&self, path: &Path) -> impl Iterator<Item = &Path>`.

- [ ] **Step 1: Write the failing tests** — append to `mod tests` in `index.rs`:

```rust
    #[test]
    fn records_the_files_that_include_a_document() {
        let guide = PathBuf::from("docs/guide.adoc");
        let setup = PathBuf::from("docs/chapters/setup.adoc");
        let mut index = WorkspaceIndex::new();
        // Indexed before its includer: the map is keyed by target, so order is irrelevant.
        index.index_source(&setup, "== Setup\n");
        index.index_source(&guide, "= Guide\n\ninclude::chapters/setup.adoc[]\n");

        let includers: Vec<_> = index.includers(&setup).collect();

        assert_eq!(includers, vec![guide.as_path()]);
        assert_eq!(index.includers(&guide).count(), 0);
    }

    #[test]
    fn records_an_include_written_with_a_declared_attribute() {
        let guide = PathBuf::from("docs/guide.adoc");
        let mut index = WorkspaceIndex::new();
        index.index_source(
            &guide,
            ":chapters: chapters\n\ninclude::{chapters}/setup.adoc[]\n",
        );

        assert_eq!(
            index.includers(Path::new("docs/chapters/setup.adoc")).count(),
            1
        );
    }

    #[test]
    fn forgets_an_include_the_includer_drops() {
        let guide = PathBuf::from("docs/guide.adoc");
        let setup = Path::new("docs/chapters/setup.adoc");
        let mut index = WorkspaceIndex::new();
        index.index_source(&guide, "include::chapters/setup.adoc[]\n");

        index.index_source(&guide, "= Guide\n");

        assert_eq!(index.includers(setup).count(), 0);
    }

    #[test]
    fn forgets_the_includes_of_a_removed_file() {
        let guide = PathBuf::from("docs/guide.adoc");
        let setup = Path::new("docs/chapters/setup.adoc");
        let mut index = WorkspaceIndex::new();
        index.index_source(&guide, "include::chapters/setup.adoc[]\n");

        index.remove(&guide);

        assert_eq!(index.includers(setup).count(), 0);
    }
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p adoc-index includ`
Expected: compile error `no method named includers`. Add a stub so the tests can fail on assertions:

```rust
    pub fn includers(&self, path: &Path) -> impl Iterator<Item = &Path> {
        let _ = path;
        std::iter::empty()
    }
```

Re-run: the two "records" tests FAIL on their assertions; the two "forgets" tests pass trivially (they guard the removal path once recording exists).

- [ ] **Step 3: Implement**

In `index.rs`, extend the imports and struct:

```rust
use std::{
    collections::{hash_map::DefaultHasher, BTreeMap, BTreeSet},
    // …unchanged…
};

use crate::{diagnostics::resolve_include_target, workspace::{collect_asciidoc_files, normalize_path}};

#[derive(Clone, Debug, Default)]
pub struct WorkspaceIndex {
    files: BTreeMap<PathBuf, FileEntry>,
    anchors: BTreeMap<AnchorKey, Vec<AnchorLocation>>,
    references: Vec<ReferenceLocation>,
    /// Include target → the files that include it. Only targets the index can resolve on its
    /// own are recorded: relative paths and attributes the including document declares.
    includers: BTreeMap<PathBuf, BTreeSet<PathBuf>>,
}
```

In `replace`, after the `self.references.extend(…)` block:

```rust
        for target in include_targets(&path, &document) {
            self.includers
                .entry(target)
                .or_default()
                .insert(path.clone());
        }
```

Replace the body of `remove`:

```rust
    pub fn remove(&mut self, path: &Path) -> Option<FileEntry> {
        let path = normalize_path(path);
        self.anchors.retain(|key, _| key.path != path);
        self.references.retain(|reference| reference.path != path);
        let removed = self.files.remove(&path)?;
        // Resolution is a pure function of the text and path, so this yields exactly the
        // targets `replace` recorded.
        for target in include_targets(&path, &removed.document) {
            if let Some(includers) = self.includers.get_mut(&target) {
                includers.remove(&path);
                if includers.is_empty() {
                    self.includers.remove(&target);
                }
            }
        }
        Some(removed)
    }
```

Replace the stub with the real query, after `references`:

```rust
    /// The files that include `path`, as far as the index can resolve their includes.
    pub fn includers(&self, path: &Path) -> impl Iterator<Item = &Path> {
        self.includers
            .get(&normalize_path(path))
            .into_iter()
            .flatten()
            .map(PathBuf::as_path)
    }
```

And a free function beside `path_to_uri`:

```rust
fn include_targets<'a>(
    path: &'a Path,
    document: &'a Document,
) -> impl Iterator<Item = PathBuf> + 'a {
    document
        .includes
        .iter()
        .filter_map(move |include| resolve_include_target(document, path, &include.target))
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p adoc-index`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add crates/adoc-index/src/index.rs
git commit -m "Record which files include each document in the index"
```

---

### Task 2: Plain-workspace image resolution and diagnostic

**Files:**
- Modify: `crates/adoc-core/src/diagnostic.rs` (new code)
- Modify: `crates/adoc-index/src/diagnostics.rs` (resolver, diagnostic, tests)
- Modify: `crates/adoc-index/src/lib.rs` (export)
- Create: `tests/fixtures/images/guide.adoc`, `tests/fixtures/images/media/present.svg`, `tests/fixtures/images/chapters/setup.adoc`

**Interfaces:**
- Consumes: `WorkspaceIndex::includers` (Task 1).
- Produces: `DiagnosticCode::UnresolvedImage` (`"adoc.unresolved-image"`); `adoc_index::resolve_image_target(document: &Document, current_path: &Path, target: &str) -> Option<PathBuf>`.

- [ ] **Step 1: Create the fixture**

`tests/fixtures/images/guide.adoc`:

```asciidoc
= Guide
:imagesdir: media

image::present.svg[]
image::missing.svg[]

include::chapters/setup.adoc[]
```

`tests/fixtures/images/media/present.svg`:

```xml
<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"/>
```

`tests/fixtures/images/chapters/setup.adoc` (its image resolves against `guide.adoc`'s `media/`, where it does not exist, but the file is included so it must not be judged):

```asciidoc
== Setup

image::screens/login.svg[]
```

- [ ] **Step 2: Write the failing tests** — append to `mod tests` in `adoc-index/src/diagnostics.rs`, and add `resolve_image_target` to that module's `use crate::{…}`:

```rust
    #[test]
    fn resolves_image_targets_against_the_document_and_its_imagesdir() {
        let path = Path::new("docs/guide.adoc");
        let plain = adoc_parser::parse("file:///docs/guide.adoc", "= Guide\n").document;
        let with_dir = adoc_parser::parse(
            "file:///docs/guide.adoc",
            ":base: assets\n:imagesdir: {base}/img\n",
        )
        .document;

        assert_eq!(
            resolve_image_target(&plain, path, "a.png"),
            Some(PathBuf::from("docs/a.png"))
        );
        assert_eq!(
            resolve_image_target(&plain, path, "/abs/a.png"),
            Some(PathBuf::from("/abs/a.png"))
        );
        assert_eq!(
            resolve_image_target(&with_dir, path, "a.png"),
            Some(PathBuf::from("docs/assets/img/a.png"))
        );
    }

    #[test]
    fn leaves_image_targets_it_cannot_judge_unresolved() {
        let path = Path::new("docs/guide.adoc");
        let document = adoc_parser::parse("file:///docs/guide.adoc", "= Guide\n").document;
        let remote_dir =
            adoc_parser::parse("file:///docs/guide.adoc", ":imagesdir: https://cdn.example\n")
                .document;

        for target in [
            "https://example.com/a.png",
            "data:image/png;base64,AAAA",
            "{undeclared}/a.png",
            "ROOT:a.png",
            "image$a.png",
        ] {
            assert_eq!(resolve_image_target(&document, path, target), None, "{target}");
        }
        assert_eq!(resolve_image_target(&remote_dir, path, "a.png"), None);
    }

    #[test]
    fn reports_a_missing_image_only_in_files_nothing_includes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/images");
        let mut index = WorkspaceIndex::new();
        index.index_roots(std::slice::from_ref(&root)).unwrap();

        let guide: Vec<_> = workspace_diagnostics(&index, &root.join("guide.adoc"))
            .into_iter()
            .filter(|diagnostic| diagnostic.code == DiagnosticCode::UnresolvedImage)
            .collect();
        assert_eq!(guide.len(), 1, "{guide:?}");
        assert!(guide[0].message.contains("missing.svg"));

        assert!(
            workspace_diagnostics(&index, &root.join("chapters/setup.adoc")).is_empty(),
            "an included file's images resolve against its includer"
        );
    }
```

Ensure the test module imports `std::path::{Path, PathBuf}`.

- [ ] **Step 3: Run them to verify they fail**

Run: `cargo test -p adoc-index image`
Expected: compile errors for `UnresolvedImage` and `resolve_image_target`. Add the enum variant and a stub returning `None` (Step 4 below shows the variant); re-run and confirm the resolver test and the fixture test FAIL on assertions.

- [ ] **Step 4: Implement**

`adoc-core/src/diagnostic.rs` — add `UnresolvedImage` after `UnresolvedInclude` in the enum, and in `as_str`:

```rust
            Self::UnresolvedImage => "adoc.unresolved-image",
```

Run `rg "DiagnosticCode::" crates` and add the variant to any other exhaustive `match`.

`adoc-index/src/diagnostics.rs` — after `resolve_include_target`:

```rust
/// The file a block image names in a plain workspace, or `None` when the index cannot judge
/// the target: a URL, a `data:` URI, an Antora resource ID, or an attribute the document does
/// not declare.
///
/// Asciidoctor resolves images against `imagesdir`, which defaults to the document's
/// directory, so a declared `imagesdir` is joined in front of the target.
#[must_use]
pub fn resolve_image_target(
    document: &Document,
    current_path: &Path,
    target: &str,
) -> Option<PathBuf> {
    // A colon covers `://`, `data:` and Antora coordinates alike.
    if target.contains(':') || target.contains('$') {
        return None;
    }
    let target = substitute_attributes(document, target)?;
    let parent = current_path.parent().unwrap_or_else(|| Path::new(""));
    let base = match document
        .attributes
        .iter()
        .rev()
        .find(|attribute| attribute.name == "imagesdir")
    {
        Some(attribute) => {
            let directory =
                substitute_attributes(document, attribute.value.as_deref().unwrap_or(""))?;
            if directory.contains(':') {
                return None;
            }
            parent.join(directory)
        }
        None => parent.to_path_buf(),
    };
    Some(normalize_path(&base.join(target)))
}
```

In `workspace_diagnostics`, after the `for include in &document.includes` loop:

```rust
    // Images resolve against the top-level document, so a file composed into another cannot
    // judge its own.
    if index.includers(path).next().is_none() {
        for image in &document.images {
            let Some(target_path) = resolve_image_target(document, path, &image.target) else {
                continue;
            };
            if !target_path.exists() && index.file(&target_path).is_none() {
                push_unique(
                    &mut diagnostics,
                    &mut seen,
                    DiagnosticCode::UnresolvedImage,
                    format!("Unresolved AsciiDoc image target: {}", image.target),
                    image.range,
                );
            }
        }
    }
```

`adoc-index/src/lib.rs`:

```rust
pub use diagnostics::{resolve_image_target, resolve_include_target, workspace_diagnostics};
```

- [ ] **Step 5: Run the tests**

Run: `cargo test --workspace`
Expected: all pass. (The `adoc-ls` Antora fixture contains no images, so its existing "no diagnostics" assertions are unaffected.)

- [ ] **Step 6: Commit**

```bash
git add crates/adoc-core/src/diagnostic.rs crates/adoc-index/src tests/fixtures/images
git commit -m "Warn about missing images in files nothing includes"
```

---

### Task 3: Antora image resolution in `adoc-ls`

**Files:**
- Create: `crates/adoc-ls/src/handlers/images.rs`
- Modify: `crates/adoc-ls/src/handlers/mod.rs` (register module)
- Modify: `crates/adoc-ls/src/handlers/diagnostics.rs` (override + tests)

**Interfaces:**
- Consumes: `DiagnosticCode::UnresolvedImage` (Task 2); `adoc_antora::{parse_resource_id, AntoraResourceId, AntoraResolver, ResourceFamily, ResolutionError}`.
- Produces: `handlers::images::antora_image_id(target: &str) -> Option<AntoraResourceId>` — used by hover later.

- [ ] **Step 1: Write the failing tests** — append to `mod tests` in `adoc-ls/src/handlers/diagnostics.rs`:

```rust
    fn antora_single_component() -> (WorkspaceIndex, adoc_antora::AntoraCatalog, std::path::PathBuf) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/antora-single-component");
        let mut index = WorkspaceIndex::new();
        index.index_roots(std::slice::from_ref(&root)).unwrap();
        let antora = discover_antora_workspace(std::slice::from_ref(&root))
            .unwrap()
            .catalog;
        (index, antora, root)
    }

    fn codes_for(
        index: &mut WorkspaceIndex,
        antora: &adoc_antora::AntoraCatalog,
        path: &Path,
        text: &str,
    ) -> Vec<DiagnosticCode> {
        index.index_source(path, text);
        diagnostics(index, antora, path)
            .into_iter()
            .map(|diagnostic| diagnostic.code)
            .collect()
    }

    #[test]
    fn resolves_antora_images_in_the_module_s_images_family() {
        let (mut index, antora, root) = antora_single_component();
        let home = root.join("modules/ROOT/pages/index.adoc");
        let security = root.join("modules/security/pages/authentication.adoc");

        // Bare, explicit-family and module-qualified forms. The index layer alone would
        // resolve the bare form against `pages/` and warn; the Antora answer replaces it.
        assert_eq!(
            codes_for(
                &mut index,
                &antora,
                &home,
                "= Home\n\nimage::architecture.svg[]\nimage::image$architecture.svg[]\n",
            ),
            Vec::new()
        );
        assert_eq!(
            codes_for(&mut index, &antora, &security, "= Auth\n\nimage::ROOT:architecture.svg[]\n"),
            Vec::new()
        );
    }

    #[test]
    fn warns_once_about_a_missing_antora_image() {
        let (mut index, antora, root) = antora_single_component();
        let home = root.join("modules/ROOT/pages/index.adoc");

        assert_eq!(
            codes_for(&mut index, &antora, &home, "= Home\n\nimage::missing.svg[]\n"),
            vec![DiagnosticCode::UnresolvedImage]
        );
    }

    #[test]
    fn reports_an_unknown_module_but_not_an_absent_component_for_images() {
        let (mut index, antora, root) = antora_single_component();
        let home = root.join("modules/ROOT/pages/index.adoc");

        assert_eq!(
            codes_for(&mut index, &antora, &home, "= Home\n\nimage::nowhere:a.svg[]\n"),
            vec![DiagnosticCode::AntoraUnknownModule]
        );
        assert_eq!(
            codes_for(&mut index, &antora, &home, "= Home\n\nimage::2.0@other:ROOT:a.svg[]\n"),
            Vec::new()
        );
    }

    #[test]
    fn leaves_images_in_partials_and_dynamic_targets_alone() {
        let (mut index, antora, root) = antora_single_component();
        let partial = root.join("modules/ROOT/partials/welcome.adoc");
        let home = root.join("modules/ROOT/pages/index.adoc");

        assert_eq!(
            codes_for(&mut index, &antora, &partial, "image::missing.svg[]\n"),
            Vec::new()
        );
        assert_eq!(
            codes_for(
                &mut index,
                &antora,
                &home,
                "= Home\n\nimage::{undeclared}/a.svg[]\nimage::https://example.com/a.svg[]\nimage::data:image/png;base64,AAAA[]\n",
            ),
            Vec::new()
        );
    }
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p adoc-ls --lib diagnostics`
Expected: `resolves_antora_images_in_the_module_s_images_family` FAILS (index-layer `UnresolvedImage` for the bare form), `warns_once_…` FAILS or passes by coincidence, `reports_an_unknown_module_…` FAILS. Note which fail and why before continuing.

- [ ] **Step 3: Implement**

`crates/adoc-ls/src/handlers/images.rs`:

```rust
//! Where an image macro's target points inside an Antora module.

use adoc_antora::{parse_resource_id, AntoraResourceId, ResourceFamily};

/// The Antora resource an image target names, defaulting to the `image` family.
///
/// `None` for targets no catalog can judge: URLs, `data:` URIs, and attribute references.
/// The resolver itself defaults a missing family to `page`, so the family is set here.
#[must_use]
pub fn antora_image_id(target: &str) -> Option<AntoraResourceId> {
    if target.contains("://") || target.starts_with("data:") || target.contains('{') {
        return None;
    }
    let mut id = parse_resource_id(target).ok()?;
    id.family.get_or_insert(ResourceFamily::Image);
    Some(id)
}
```

`handlers/mod.rs`: add `pub mod images;` in alphabetical order.

`handlers/diagnostics.rs` — import `super::images::antora_image_id`. After the existing `diagnostics.retain(…)` that drops resolved xref/anchor findings, add:

```rust
    // The index layer resolved images against the document's directory, which is wrong
    // inside a module: Antora resolves them in the module's `images` family instead.
    diagnostics.retain(|diagnostic| diagnostic.code != DiagnosticCode::UnresolvedImage);
```

Inside the `{ let mut output = …; }` block, after the include loop:

```rust
        // As with anchors, a partial is judged only as part of the page that includes it.
        if !composed_elsewhere {
            for image in &file.document.images {
                let Some(id) = antora_image_id(&image.target) else {
                    continue;
                };
                match AntoraResolver::resolve(antora, &id, &context) {
                    // A component missing from the workspace is published from elsewhere.
                    Ok(_) | Err(ResolutionError::UnknownComponent { .. }) => {}
                    Err(ResolutionError::UnknownModule { module, .. }) => output.push(
                        DiagnosticCode::AntoraUnknownModule,
                        format!("Unknown Antora module: {module}"),
                        image.range,
                    ),
                    Err(ResolutionError::UnknownResource { .. }) => output.push(
                        DiagnosticCode::UnresolvedImage,
                        format!("Unresolved AsciiDoc image target: {}", image.target),
                        image.range,
                    ),
                }
            }
        }
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --workspace`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add crates/adoc-ls/src/handlers
git commit -m "Resolve Antora images in the module's images family"
```

---

### Task 4: Documentation and verification

**Files:**
- Modify: `README.md` (Implemented list)
- Add: `docs/prd/DESIGN-Missing_Image_Diagnostic.md`, this plan

- [ ] **Step 1: README** — in the "Implemented" list, extend the diagnostics bullet:

```markdown
- Workspace indexing and conservative diagnostics for missing local xrefs, anchors, includes,
  and block images. An image is judged only in a file nothing includes, since Asciidoctor
  resolves it against the top-level document; inside an Antora module it is resolved as an
  `image$` resource of the page's module, and partials are not judged on their own.
```

- [ ] **Step 2: Full verification**

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p asciidoc-zed-extension --target wasm32-wasip2
cargo run -p adoc-ls -- --version
```

Expected: all succeed with no warnings.

- [ ] **Step 3: Commit**

```bash
git add README.md docs/prd/DESIGN-Missing_Image_Diagnostic.md docs/superpowers/plans/2026-10-01-missing-image-diagnostic.md
git commit -m "Document the missing-image diagnostic"
```
