//! What the server advertises in its `initialize` response. The Lua and VS Code
//! clients depend on this shape.

use tower_lsp::lsp_types::*;

use crate::lsp::commands::SUPPORTED_COMMANDS;
use crate::lsp::semantic_token::LEGEND_TYPE;

const COMPLETION_TRIGGER_CHARACTERS: &[&str] =
    &["[", "#", "@img", "@math", "@quote", "@table", "@task"];

pub(super) fn server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        // vscode only supports UTF-16.
        position_encoding: Some(PositionEncodingKind::UTF16),
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        completion_provider: Some(CompletionOptions {
            resolve_provider: Some(false),
            trigger_characters: Some(
                COMPLETION_TRIGGER_CHARACTERS
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
            ),
            work_done_progress_options: Default::default(),
            all_commit_characters: None,
            ..Default::default()
        }),
        execute_command_provider: Some(ExecuteCommandOptions {
            commands: SUPPORTED_COMMANDS.iter().map(ToString::to_string).collect(),
            work_done_progress_options: Default::default(),
        }),
        workspace: Some(WorkspaceServerCapabilities {
            workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                supported: Some(true),
                change_notifications: Some(OneOf::Left(true)),
            }),
            file_operations: None,
        }),
        semantic_tokens_provider: Some(semantic_tokens_capability()),
        definition_provider: Some(OneOf::Left(true)),
        references_provider: Some(OneOf::Left(true)),
        rename_provider: Some(OneOf::Right(RenameOptions {
            prepare_provider: Some(true),
            work_done_progress_options: Default::default(),
        })),
        folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
        ..ServerCapabilities::default()
    }
}

fn semantic_tokens_capability() -> SemanticTokensServerCapabilities {
    SemanticTokensServerCapabilities::SemanticTokensRegistrationOptions(
        SemanticTokensRegistrationOptions {
            text_document_registration_options: TextDocumentRegistrationOptions {
                document_selector: Some(vec![DocumentFilter {
                    language: Some("patto".to_string()),
                    scheme: Some("file".to_string()),
                    pattern: None,
                }]),
            },
            semantic_tokens_options: SemanticTokensOptions {
                work_done_progress_options: WorkDoneProgressOptions::default(),
                legend: SemanticTokensLegend {
                    token_types: LEGEND_TYPE.into(),
                    token_modifiers: vec![],
                },
                range: Some(true),
                full: Some(SemanticTokensFullOptions::Bool(true)),
            },
            static_registration_options: StaticRegistrationOptions::default(),
        },
    )
}
