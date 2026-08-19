/// Renders the document once.
pub const RENDER_PREVIEW_COMMAND: &str = "adoc.renderPreview";

/// Renders the document and keeps the artefact up to date as the document is saved.
pub const RENDER_LIVE_PREVIEW_COMMAND: &str = "adoc.renderLivePreview";

/// An editor-agnostic code action. `protocol.rs` maps this onto `lsp_types::CodeAction`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewAction {
    pub title: String,
    pub command: &'static str,
    pub uri: String,
}

#[must_use]
pub fn code_actions_for(uri: &str) -> Vec<PreviewAction> {
    vec![
        PreviewAction {
            title: "AsciiDoc: render preview".to_owned(),
            command: RENDER_PREVIEW_COMMAND,
            uri: uri.to_owned(),
        },
        PreviewAction {
            title: "AsciiDoc: render live preview".to_owned(),
            command: RENDER_LIVE_PREVIEW_COMMAND,
            uri: uri.to_owned(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offers_both_a_one_off_and_a_live_preview() {
        let actions = code_actions_for("file:///docs/guide.adoc");

        let commands: Vec<_> = actions.iter().map(|action| action.command).collect();
        assert_eq!(
            commands,
            vec![RENDER_PREVIEW_COMMAND, RENDER_LIVE_PREVIEW_COMMAND]
        );
        assert!(actions.iter().all(|a| a.uri == "file:///docs/guide.adoc"));
    }

    #[test]
    fn the_two_actions_are_distinguishable_in_a_menu() {
        let actions = code_actions_for("file:///docs/guide.adoc");

        assert_ne!(actions[0].title, actions[1].title);
        assert!(actions[1].title.contains("live"), "{}", actions[1].title);
    }

    #[test]
    fn preview_action_title_names_the_operation() {
        let actions = code_actions_for("file:///docs/guide.adoc");

        assert!(actions[0].title.contains("preview"), "{}", actions[0].title);
    }
}
