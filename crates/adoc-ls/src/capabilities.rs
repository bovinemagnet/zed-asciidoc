use lsp_types::{
    CodeActionProviderCapability, ExecuteCommandOptions, OneOf, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    TextDocumentSyncSaveOptions,
};

use crate::handlers::code_actions::{
    RENDER_LIVE_PREVIEW_COMMAND, RENDER_PREVIEW_COMMAND, STOP_LIVE_PREVIEW_COMMAND,
};

#[must_use]
pub(crate) fn server_capabilities(
    encoding: crate::position::PositionEncoding,
) -> ServerCapabilities {
    ServerCapabilities {
        position_encoding: Some(encoding.lsp_kind()),
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::INCREMENTAL),
                // A live preview re-renders on save, which needs the notification.
                save: Some(TextDocumentSyncSaveOptions::Supported(true)),
                ..TextDocumentSyncOptions::default()
            },
        )),
        document_symbol_provider: Some(OneOf::Left(true)),
        definition_provider: Some(OneOf::Left(true)),
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
        execute_command_provider: Some(ExecuteCommandOptions {
            // A client may drop a code action whose command is missing from this list —
            // Zed does — so every command a code action carries belongs here.
            commands: vec![
                RENDER_PREVIEW_COMMAND.to_owned(),
                RENDER_LIVE_PREVIEW_COMMAND.to_owned(),
                STOP_LIVE_PREVIEW_COMMAND.to_owned(),
            ],
            ..ExecuteCommandOptions::default()
        }),
        ..ServerCapabilities::default()
    }
}
