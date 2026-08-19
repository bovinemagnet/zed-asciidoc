//! Handling of `workspace/executeCommand`.
//!
//! Editor-agnostic: this operates on a URI and returns the artefact that was produced,
//! leaving all `lsp-types` mapping to `protocol.rs`.

use std::path::{Path, PathBuf};

use adoc_render::{RenderRequest, Renderer};

use crate::{
    preview::{PreviewError, PreviewSink},
    state::{document_path, ServerState},
};

/// Render the document at `uri` and deliver it through `sink`.
///
/// The open buffer is rendered rather than the file on disk, so an unsaved document
/// previews exactly what the author sees.
pub fn render_preview(
    state: &ServerState,
    renderer: &dyn Renderer,
    sink: &dyn PreviewSink,
    uri: &str,
) -> Result<PathBuf, PreviewError> {
    let open = state
        .documents
        .get(uri)
        .ok_or_else(|| PreviewError::NotOpen(uri.to_owned()))?;

    let source = document_path(uri);
    let mut request = RenderRequest::from_source(source.clone(), open.document.text.clone());
    request.attributes.extend(antora_attributes(state, &source));

    let output = renderer.render(&request)?;
    sink.deliver(&output, &source)
}

/// Antora page attributes for `source`, empty when the file is not in an Antora module.
///
/// `adoc-ls` knows the component and module; a bare Asciidoctor invocation would not.
fn antora_attributes(
    state: &ServerState,
    source: &Path,
) -> impl Iterator<Item = (String, String)> + use<> {
    let context = state.antora.context_for_path(source);
    context.into_iter().flat_map(|context| {
        let mut attributes = vec![
            ("page-component-name".to_owned(), context.component),
            ("page-module".to_owned(), context.module),
        ];
        if let Some(version) = context.version {
            attributes.push(("page-component-version".to_owned(), version));
        }
        attributes
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use adoc_render::MockRenderer;
    use std::sync::Mutex;

    #[derive(Default)]
    struct RecordingSink {
        delivered: Mutex<Vec<(String, PathBuf)>>,
    }

    impl PreviewSink for RecordingSink {
        fn deliver(
            &self,
            output: &adoc_render::RenderOutput,
            source: &Path,
        ) -> Result<PathBuf, PreviewError> {
            self.delivered
                .lock()
                .unwrap()
                .push((output.html.clone(), source.to_path_buf()));
            Ok(source.with_extension("html"))
        }
    }

    fn state_with(uri: &str, text: &str) -> ServerState {
        let mut state = ServerState::default();
        state.open(uri, text, 1);
        state
    }

    #[test]
    fn renders_the_unsaved_buffer_rather_than_the_file_on_disk() {
        let uri = "file:///docs/guide.adoc";
        let state = state_with(uri, "= Unsaved Title\n");
        let renderer = RecordingRenderer::default();
        let sink = RecordingSink::default();

        render_preview(&state, &renderer, &sink, uri).expect("render");

        let request = renderer.last_request().expect("a request was made");
        assert_eq!(request.source_text.as_deref(), Some("= Unsaved Title\n"));
    }

    #[test]
    fn delivers_rendered_html_for_the_document_path() {
        let uri = "file:///docs/guide.adoc";
        let state = state_with(uri, "= Title\n");
        let sink = RecordingSink::default();

        render_preview(&state, &MockRenderer::new("<h1>Title</h1>"), &sink, uri).expect("render");

        let delivered = sink.delivered.lock().unwrap();
        assert_eq!(delivered.len(), 1);
        assert_eq!(delivered[0].0, "<h1>Title</h1>");
        assert_eq!(delivered[0].1, PathBuf::from("/docs/guide.adoc"));
    }

    #[test]
    fn reports_a_document_that_is_not_open() {
        let state = ServerState::default();
        let sink = RecordingSink::default();

        let error = render_preview(
            &state,
            &MockRenderer::new("<p/>"),
            &sink,
            "file:///gone.adoc",
        )
        .expect_err("closed documents cannot be previewed");

        assert!(matches!(error, PreviewError::NotOpen(_)), "{error:?}");
    }

    #[test]
    fn returns_the_artefact_path_from_the_sink() {
        let uri = "file:///docs/guide.adoc";
        let state = state_with(uri, "= Title\n");

        let artefact = render_preview(
            &state,
            &MockRenderer::new("<h1>Title</h1>"),
            &RecordingSink::default(),
            uri,
        )
        .expect("render");

        assert_eq!(artefact, PathBuf::from("/docs/guide.html"));
    }

    #[test]
    fn merges_antora_page_attributes_into_the_request() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/antora-single-component");
        let page = root.join("modules/security/pages/authentication.adoc");
        let uri = format!("file://{}", page.display());

        let mut state = ServerState::default();
        state.index_workspace(vec![root]).expect("index fixture");
        state.open(&uri, "= Authentication\n", 1);

        let renderer = RecordingRenderer::default();
        render_preview(&state, &renderer, &RecordingSink::default(), &uri).expect("render");

        let attributes = renderer.last_request().expect("a request").attributes;
        assert_eq!(
            attributes.get("page-component-name").map(String::as_str),
            Some("demo")
        );
        assert_eq!(
            attributes.get("page-module").map(String::as_str),
            Some("security")
        );
        assert_eq!(
            attributes.get("page-component-version").map(String::as_str),
            Some("latest")
        );
    }

    #[test]
    fn a_document_outside_an_antora_module_contributes_no_page_attributes() {
        let uri = "file:///elsewhere/guide.adoc";
        let state = state_with(uri, "= Title\n");

        let renderer = RecordingRenderer::default();
        render_preview(&state, &renderer, &RecordingSink::default(), uri).expect("render");

        assert!(renderer
            .last_request()
            .expect("a request")
            .attributes
            .is_empty());
    }

    /// A renderer that captures the request it was given.
    #[derive(Default)]
    struct RecordingRenderer {
        requests: Mutex<Vec<RenderRequest>>,
    }

    impl RecordingRenderer {
        fn last_request(&self) -> Option<RenderRequest> {
            self.requests.lock().unwrap().last().cloned()
        }
    }

    impl adoc_render::Renderer for RecordingRenderer {
        fn render(
            &self,
            request: &RenderRequest,
        ) -> Result<adoc_render::RenderOutput, adoc_render::RenderError> {
            self.requests.lock().unwrap().push(request.clone());
            Ok(adoc_render::RenderOutput {
                html: String::new(),
                warnings: Vec::new(),
            })
        }
    }
}
