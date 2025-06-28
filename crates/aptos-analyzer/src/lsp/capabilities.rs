use crate::config::Config;
use crate::line_index::PositionEncoding;
use crate::lsp::semantic_tokens;
use ide::inlay_hints::InlayFieldsToResolve;
use line_index::WideEncoding;
use lsp_types::{
    CodeActionKind, CodeActionOptions, CodeActionProviderCapability, CompletionOptions,
    CompletionOptionsCompletionItem, HoverProviderCapability, InlayHintOptions,
    InlayHintServerCapabilities, OneOf, PositionEncodingKind, RenameOptions,
    SelectionRangeProviderCapability, SemanticTokensFullOptions, SemanticTokensLegend,
    SemanticTokensOptions, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
    WorkDoneProgressOptions,
};
use std::collections::HashSet;

pub fn server_capabilities(config: &Config) -> ServerCapabilities {
    ServerCapabilities {
        position_encoding: match config.caps().negotiated_encoding() {
            PositionEncoding::Utf8 => Some(PositionEncodingKind::UTF8),
            PositionEncoding::Wide(wide) => match wide {
                WideEncoding::Utf16 => Some(PositionEncodingKind::UTF16),
                WideEncoding::Utf32 => Some(PositionEncodingKind::UTF32),
                _ => None,
            },
        },
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        // text_document_sync: Some(TextDocumentSyncCapability::Options(TextDocumentSyncOptions {
        //     open_close: Some(true),
        //     change: Some(TextDocumentSyncKind::INCREMENTAL),
        //     will_save: None,
        //     will_save_wait_until: None,
        //     save: Some(SaveOptions::default().into()),
        // })),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        completion_provider: Some(CompletionOptions {
            resolve_provider: None,
            // resolve_provider: Some(config.caps().completions_resolve_provider()),
            trigger_characters: Some(vec![
                ":".to_owned(),
                ".".to_owned(),
                "'".to_owned(),
                "(".to_owned(),
            ]),
            all_commit_characters: None,
            completion_item: None,
            // completion_item: config.caps().completion_item(),
            work_done_progress_options: WorkDoneProgressOptions { work_done_progress: None },
        }),
        // signature_help_provider: Some(SignatureHelpOptions {
        //     trigger_characters: Some(vec!["(".to_owned(), ",".to_owned(), "<".to_owned()]),
        //     retrigger_characters: None,
        //     work_done_progress_options: WorkDoneProgressOptions { work_done_progress: None },
        // }),
        // declaration_provider: Some(DeclarationCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        // type_definition_provider: Some(TypeDefinitionProviderCapability::Simple(true)),
        // implementation_provider: Some(ImplementationProviderCapability::Simple(true)),
        references_provider: Some(OneOf::Left(true)),
        document_highlight_provider: Some(OneOf::Left(true)),
        // document_symbol_provider: Some(OneOf::Left(true)),
        workspace_symbol_provider: Some(OneOf::Left(true)),
        code_action_provider: Some(config.caps().code_action_capabilities()),
        // code_lens_provider: Some(CodeLensOptions { resolve_provider: Some(true) }),
        document_formatting_provider: Some(OneOf::Left(true)),
        // document_range_formatting_provider: match config.rustfmt(None) {
        //     RustfmtConfig::Rustfmt { enable_range_formatting: true, .. } => Some(OneOf::Left(true)),
        //     _ => Some(OneOf::Left(false)),
        // },
        // document_on_type_formatting_provider: Some({
        //     let mut chars = ide::Analysis::SUPPORTED_TRIGGER_CHARS.chars();
        //     DocumentOnTypeFormattingOptions {
        //         first_trigger_character: chars.next().unwrap().to_string(),
        //         more_trigger_character: Some(chars.map(|c| c.to_string()).collect()),
        //     }
        // }),
        selection_range_provider: Some(SelectionRangeProviderCapability::Simple(true)),
        // folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
        rename_provider: Some(OneOf::Right(RenameOptions {
            prepare_provider: Some(true),
            work_done_progress_options: WorkDoneProgressOptions { work_done_progress: None },
        })),
        // linked_editing_range_provider: None,
        // document_link_provider: None,
        // color_provider: None,
        // execute_command_provider: None,
        // workspace: Some(WorkspaceServerCapabilities {
        //     workspace_folders: Some(WorkspaceFoldersServerCapabilities {
        //         supported: Some(true),
        //         change_notifications: Some(OneOf::Left(true)),
        //     }),
        //     file_operations: Some(WorkspaceFileOperationsServerCapabilities {
        //         did_create: None,
        //         will_create: None,
        //         did_rename: None,
        //         will_rename: Some(FileOperationRegistrationOptions {
        //             filters: vec![
        //                 FileOperationFilter {
        //                     scheme: Some(String::from("file")),
        //                     pattern: FileOperationPattern {
        //                         glob: String::from("**/*.rs"),
        //                         matches: Some(FileOperationPatternKind::File),
        //                         options: None,
        //                     },
        //                 },
        //                 FileOperationFilter {
        //                     scheme: Some(String::from("file")),
        //                     pattern: FileOperationPattern {
        //                         glob: String::from("**"),
        //                         matches: Some(FileOperationPatternKind::Folder),
        //                         options: None,
        //                     },
        //                 },
        //             ],
        //         }),
        //         did_delete: None,
        //         will_delete: None,
        //     }),
        // }),
        // call_hierarchy_provider: Some(CallHierarchyServerCapability::Simple(true)),
        semantic_tokens_provider: Some(
            SemanticTokensOptions {
                legend: SemanticTokensLegend {
                    token_types: semantic_tokens::SUPPORTED_TYPES.to_vec(),
                    token_modifiers: vec![],
                    // token_modifiers: semantic_tokens::SUPPORTED_MODIFIERS.to_vec(),
                },

                full: Some(SemanticTokensFullOptions::Bool(true)),
                // full: Some(SemanticTokensFullOptions::Delta { delta: Some(true) }),
                range: None,
                // range: Some(true),
                work_done_progress_options: Default::default(),
            }
            .into(),
        ),
        // moniker_provider: None,
        // inlay_hint_provider: Some(OneOf::Left(true)),
        inlay_hint_provider: Some(OneOf::Right(InlayHintServerCapabilities::Options(
            InlayHintOptions {
                work_done_progress_options: Default::default(),
                resolve_provider: Some(config.caps().inlay_hints_resolve_provider()),
            },
        ))),
        // inline_value_provider: None,
        // experimental: Some(json!({
        //     "externalDocs": true,
        //     "hoverRange": true,
        //     "joinLines": true,
        //     "matchingBrace": true,
        //     "moveItem": true,
        //     "onEnter": true,
        //     "openCargoToml": true,
        //     "parentModule": true,
        //     "runnables": {
        //         "kinds": [ "cargo" ],
        //     },
        //     "ssr": true,
        //     "workspaceSymbolScopeKindFiltering": true,
        // })),
        diagnostic_provider: Some(lsp_types::DiagnosticServerCapabilities::Options(
            lsp_types::DiagnosticOptions {
                identifier: None,
                inter_file_dependencies: true,
                // FIXME
                workspace_diagnostics: false,
                work_done_progress_options: WorkDoneProgressOptions { work_done_progress: None },
            },
        )),
        // inline_completion_provider: None,
        ..ServerCapabilities::default()
    }
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct ClientCapabilities(pub lsp_types::ClientCapabilities);

impl ClientCapabilities {
    pub fn new(caps: lsp_types::ClientCapabilities) -> Self {
        Self(caps)
    }

    pub fn inlay_hint_resolve_support_properties(&self) -> HashSet<&str> {
        self.0
            .text_document
            .as_ref()
            .and_then(|text| text.inlay_hint.as_ref())
            .and_then(|inlay_hint_caps| inlay_hint_caps.resolve_support.as_ref())
            .map(|inlay_resolve| inlay_resolve.properties.iter())
            .into_iter()
            .flatten()
            .map(|s| s.as_str())
            .collect()
    }

    fn inlay_hints_resolve_provider(&self) -> bool {
        let client_capabilities = self.inlay_hint_resolve_support_properties();
        let fields_to_resolve = InlayFieldsToResolve::from_client_capabilities(&client_capabilities);
        fields_to_resolve != InlayFieldsToResolve::empty()
    }

    pub fn location_link(&self) -> bool {
        (|| -> _ { self.0.text_document.as_ref()?.definition?.link_support })().unwrap_or_default()
    }

    pub fn semantic_tokens_refresh(&self) -> bool {
        (|| -> _ {
            self.0
                .workspace
                .as_ref()?
                .semantic_tokens
                .as_ref()?
                .refresh_support
        })()
        .unwrap_or_default()
    }

    pub fn code_lens_refresh(&self) -> bool {
        (|| -> _ { self.0.workspace.as_ref()?.code_lens.as_ref()?.refresh_support })()
            .unwrap_or_default()
    }

    pub fn inlay_hints_refresh(&self) -> bool {
        (|| -> _ { self.0.workspace.as_ref()?.inlay_hint.as_ref()?.refresh_support })()
            .unwrap_or_default()
    }

    pub fn diagnostics_refresh(&self) -> bool {
        (|| -> _ { self.0.workspace.as_ref()?.diagnostic.as_ref()?.refresh_support })()
            .unwrap_or_default()
    }

    pub fn completion_label_details_support(&self) -> bool {
        (|| -> _ {
            self.0
                .text_document
                .as_ref()?
                .completion
                .as_ref()?
                .completion_item
                .as_ref()?
                .label_details_support
        })() == Some(true)
    }

    fn completion_item(&self) -> Option<CompletionOptionsCompletionItem> {
        Some(CompletionOptionsCompletionItem {
            label_details_support: Some(self.completion_label_details_support()),
        })
    }

    fn code_action_capabilities(&self) -> CodeActionProviderCapability {
        self.0
            .text_document
            .as_ref()
            .and_then(|it| it.code_action.as_ref())
            .and_then(|it| it.code_action_literal_support.as_ref())
            .map_or(CodeActionProviderCapability::Simple(true), |_| {
                CodeActionProviderCapability::Options(CodeActionOptions {
                    // Advertise support for all built-in CodeActionKinds.
                    // Ideally we would base this off of the client capabilities
                    // but the client is supposed to fall back gracefully for unknown values.
                    code_action_kinds: Some(vec![
                        // CodeActionKind::EMPTY,
                        CodeActionKind::QUICKFIX,
                        // CodeActionKind::REFACTOR,
                        // CodeActionKind::REFACTOR_EXTRACT,
                        // CodeActionKind::REFACTOR_INLINE,
                        // CodeActionKind::REFACTOR_REWRITE,
                    ]),
                    resolve_provider: Some(true),
                    work_done_progress_options: Default::default(),
                })
            })
    }

    pub fn negotiated_encoding(&self) -> PositionEncoding {
        let client_encodings = match &self.0.general {
            Some(general) => general.position_encodings.as_deref().unwrap_or_default(),
            None => &[],
        };

        for enc in client_encodings {
            if enc == &PositionEncodingKind::UTF8 {
                return PositionEncoding::Utf8;
            } else if enc == &PositionEncodingKind::UTF32 {
                return PositionEncoding::Wide(WideEncoding::Utf32);
            }
            // NB: intentionally prefer just about anything else to utf-16.
        }

        PositionEncoding::Wide(WideEncoding::Utf16)
    }

    pub fn workspace_edit_resource_operations(&self) -> Option<&[lsp_types::ResourceOperationKind]> {
        self.0
            .workspace
            .as_ref()?
            .workspace_edit
            .as_ref()?
            .resource_operations
            .as_deref()
    }

    pub fn insert_replace_support(&self) -> bool {
        (|| -> _ {
            self.0
                .text_document
                .as_ref()?
                .completion
                .as_ref()?
                .completion_item
                .as_ref()?
                .insert_replace_support
        })()
        .unwrap_or_default()
    }

    pub fn completion_snippet(&self) -> bool {
        (|| -> _ {
            self.0
                .text_document
                .as_ref()?
                .completion
                .as_ref()?
                .completion_item
                .as_ref()?
                .snippet_support
        })()
        .unwrap_or_default()
    }

    pub fn did_save_text_document_dynamic_registration(&self) -> bool {
        let caps =
            (|| -> _ { self.0.text_document.as_ref()?.synchronization.clone() })().unwrap_or_default();
        caps.did_save == Some(true) && caps.dynamic_registration == Some(true)
    }

    pub fn did_change_watched_files_dynamic_registration(&self) -> bool {
        (|| -> _ {
            self.0
                .workspace
                .as_ref()?
                .did_change_watched_files
                .as_ref()?
                .dynamic_registration
        })()
        .unwrap_or_default()
    }

    pub fn did_change_watched_files_relative_pattern_support(&self) -> bool {
        (|| -> _ {
            self.0
                .workspace
                .as_ref()?
                .did_change_watched_files
                .as_ref()?
                .relative_pattern_support
        })()
        .unwrap_or_default()
    }

    pub fn code_action_literals(&self) -> bool {
        (|| -> _ {
            self.0
                .text_document
                .as_ref()?
                .code_action
                .as_ref()?
                .code_action_literal_support
                .as_ref()
        })()
        .is_some()
    }

    pub fn work_done_progress(&self) -> bool {
        (|| -> _ { self.0.window.as_ref()?.work_done_progress })().unwrap_or_default()
    }

    pub fn will_rename(&self) -> bool {
        (|| -> _ { self.0.workspace.as_ref()?.file_operations.as_ref()?.will_rename })()
            .unwrap_or_default()
    }

    pub fn change_annotation_support(&self) -> bool {
        (|| -> _ {
            self.0
                .workspace
                .as_ref()?
                .workspace_edit
                .as_ref()?
                .change_annotation_support
                .as_ref()
        })()
        .is_some()
    }

    pub fn code_action_resolve(&self) -> bool {
        (|| -> _ {
            Some(
                self.0
                    .text_document
                    .as_ref()?
                    .code_action
                    .as_ref()?
                    .resolve_support
                    .as_ref()?
                    .properties
                    .as_slice(),
            )
        })()
        .unwrap_or_default()
        .iter()
        .any(|it| it == "edit")
    }

    pub fn signature_help_label_offsets(&self) -> bool {
        (|| -> _ {
            self.0
                .text_document
                .as_ref()?
                .signature_help
                .as_ref()?
                .signature_information
                .as_ref()?
                .parameter_information
                .as_ref()?
                .label_offset_support
        })()
        .unwrap_or_default()
    }

    pub fn text_document_diagnostic_pull_enabled(&self) -> bool {
        (|| -> _ { self.0.text_document.as_ref()?.diagnostic.as_ref() })().is_some()
    }

    pub fn text_document_diagnostic_related_document_support(&self) -> bool {
        (|| -> _ {
            self.0
                .text_document
                .as_ref()?
                .diagnostic
                .as_ref()?
                .related_document_support
        })() == Some(true)
    }

    pub fn code_action_group(&self) -> bool {
        self.experimental_bool("codeActionGroup")
    }

    pub fn open_server_logs(&self) -> bool {
        self.experimental_bool("openServerLogs")
    }

    pub fn server_status_notification(&self) -> bool {
        self.experimental_bool("serverStatusNotification")
    }

    pub fn experimental_bool(&self, index: &'static str) -> bool {
        || -> _ { self.0.experimental.as_ref()?.get(index)?.as_bool() }().unwrap_or_default()
    }
}
