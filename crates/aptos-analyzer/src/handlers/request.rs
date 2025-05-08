use crate::diagnostics::convert_diagnostic;
use crate::global_state::GlobalStateSnapshot;
use crate::lsp::utils::invalid_params_error;
use crate::lsp::{LspError, from_proto, to_proto};
use crate::movefmt::run_movefmt;
use crate::{Config, lsp_ext, unwrap_or_return_default};
use ide::Cancellable;
use ide_db::assists::{AssistKind, AssistResolveStrategy, SingleResolve};
use line_index::TextRange;
use lsp_server::ErrorCode;
use lsp_types::{
    HoverContents, InlayHint, InlayHintParams, ResourceOp, ResourceOperationKind, SemanticTokensParams,
    SemanticTokensRangeParams, SemanticTokensRangeResult, SemanticTokensResult, TextDocumentIdentifier,
};
use stdx::format_to;
use syntax::files::FileRange;
// pub(crate) fn handle_workspace_reload(state: &mut GlobalState, _: ()) -> anyhow::Result<()> {
//     let req = FetchPackagesRequest { force_reload_deps: false };
//     state
//         .fetch_packages_queue
//         .request_op("reload workspace request".to_owned(), req);
//     Ok(())
// }

pub(crate) fn handle_semantic_tokens_range(
    snap: GlobalStateSnapshot,
    params: SemanticTokensRangeParams,
) -> anyhow::Result<Option<SemanticTokensRangeResult>> {
    let _p = tracing::info_span!("handle_semantic_tokens_range").entered();

    let frange = unwrap_or_return_default!(from_proto::file_range(
        &snap,
        &params.text_document,
        params.range
    )?);
    let text = snap.analysis.file_text(frange.file_id)?;
    let line_index = snap.file_line_index(frange.file_id)?;

    // let mut highlight_config = snap.config.highlighting_config();
    // Avoid flashing a bunch of unresolved references when the proc-macro servers haven't been spawned yet.
    // highlight_config.syntactic_name_ref_highlighting =
    //     snap.workspaces.is_empty() || !snap.proc_macros_loaded;

    let highlights = snap.analysis.highlight_range(/*highlight_config, */ frange)?;
    let semantic_tokens = to_proto::semantic_tokens(
        &text,
        &line_index,
        highlights,
        // snap.config.semantics_tokens_augments_syntax_tokens(),
        // snap.config.highlighting_non_standard_tokens(),
    );
    Ok(Some(semantic_tokens.into()))
}

pub(crate) fn handle_semantic_tokens_full(
    snap: GlobalStateSnapshot,
    params: SemanticTokensParams,
) -> anyhow::Result<Option<SemanticTokensResult>> {
    let _p = tracing::info_span!("handle_semantic_tokens_full").entered();

    let file_id = from_proto::file_id(&snap, &params.text_document.uri)?;
    let text = snap.analysis.file_text(file_id)?;
    let line_index = snap.file_line_index(file_id)?;

    // let mut highlight_config = snap.config.highlighting_config();

    let highlights = snap.analysis.highlight(/*highlight_config, */ file_id)?;
    let semantic_tokens = to_proto::semantic_tokens(
        &text,
        &line_index,
        highlights,
        // snap.config.semantics_tokens_augments_syntax_tokens(),
        // snap.config.highlighting_non_standard_tokens(),
    );

    // Unconditionally cache the tokens
    // snap.semantic_tokens_cache.lock().insert(params.text_document.uri, semantic_tokens.clone());

    Ok(Some(semantic_tokens.into()))
}

pub(crate) fn handle_goto_definition(
    snap: GlobalStateSnapshot,
    params: lsp_types::GotoDefinitionParams,
) -> anyhow::Result<Option<lsp_types::GotoDefinitionResponse>> {
    let _p = tracing::info_span!("handle_goto_definition").entered();
    let position = from_proto::file_position(&snap, params.text_document_position_params)?;
    let nav_info = match snap.analysis.goto_definition(position)? {
        None => return Ok(None),
        Some(it) => it,
    };
    let src = FileRange {
        file_id: position.file_id,
        range: nav_info.range,
    };
    let res = to_proto::goto_definition_response(&snap, Some(src), vec![nav_info.info])?;
    Ok(Some(res))
}

pub(crate) fn handle_completion(
    snap: GlobalStateSnapshot,
    lsp_types::CompletionParams {
        text_document_position,
        context,
        ..
    }: lsp_types::CompletionParams,
) -> anyhow::Result<Option<lsp_types::CompletionResponse>> {
    let _p = tracing::info_span!("handle_completion").entered();
    let mut position = from_proto::file_position(&snap, text_document_position.clone())?;
    let line_index = snap.file_line_index(position.file_id)?;
    let completion_trigger_character = context
        .and_then(|ctx| ctx.trigger_character)
        .and_then(|s| s.chars().next());

    // let source_root = snap.analysis.source_root_id(position.file_id)?;
    let completion_config = &snap.config.completion(/*Some(source_root)*/);
    // FIXME: We should fix up the position when retrying the cancelled request instead
    position.offset = position.offset.min(line_index.index.len());
    let items = match snap.analysis.completions(
        completion_config,
        position,
        // completion_trigger_character,
    )? {
        None => return Ok(None),
        Some(items) => items,
    };

    let items = to_proto::completion_items(
        &snap.config,
        // &completion_config.fields_to_resolve,
        &line_index,
        snap.file_version(position.file_id),
        text_document_position,
        completion_trigger_character,
        items,
    );

    let completion_list = lsp_types::CompletionList { is_incomplete: true, items };
    Ok(Some(completion_list.into()))
}

pub(crate) fn handle_document_diagnostics(
    snap: GlobalStateSnapshot,
    params: lsp_types::DocumentDiagnosticParams,
) -> anyhow::Result<lsp_types::DocumentDiagnosticReportResult> {
    let file_id = from_proto::file_id(&snap, &params.text_document.uri)?;
    let config = snap.config.diagnostics_config();
    if !config.enabled {
        return Ok(empty_diagnostic_report());
    }
    let line_index = snap.file_line_index(file_id)?;
    // let supports_related = false;
    // let supports_related = snap.config.text_document_diagnostic_related_document_support();

    // let mut related_documents = FxHashMap::default();
    let diagnostics = snap
        .analysis
        .full_diagnostics(&config, AssistResolveStrategy::None, file_id)?
        .into_iter()
        .filter_map(|d| {
            let file = d.range.file_id;
            if file == file_id {
                let diagnostic = convert_diagnostic(&line_index, d);
                return Some(diagnostic);
            }
            // if supports_related {
            //     let (diagnostics, line_index) = related_documents
            //         .entry(file)
            //         .or_insert_with(|| (Vec::new(), snap.file_line_index(file).ok()));
            //     let diagnostic = convert_diagnostic(line_index.as_mut()?, d);
            //     diagnostics.push(diagnostic);
            // }
            None
        });
    Ok(lsp_types::DocumentDiagnosticReportResult::Report(
        lsp_types::DocumentDiagnosticReport::Full(lsp_types::RelatedFullDocumentDiagnosticReport {
            full_document_diagnostic_report: lsp_types::FullDocumentDiagnosticReport {
                result_id: Some("aptos-analyzer".to_owned()),
                items: diagnostics.collect(),
            },
            related_documents: None,
            // related_documents: related_documents.is_empty().not().then(|| {
            //     related_documents
            //         .into_iter()
            //         .map(|(id, (items, _))| {
            //             (
            //                 to_proto::url(&snap, id),
            //                 lsp_types::DocumentDiagnosticReportKind::Full(
            //                     lsp_types::FullDocumentDiagnosticReport {
            //                         result_id: Some("aptos-analyzer".to_owned()),
            //                         items,
            //                     },
            //                 ),
            //             )
            //         })
            //         .collect()
            // }),
        }),
    ))
}

pub(crate) fn empty_diagnostic_report() -> lsp_types::DocumentDiagnosticReportResult {
    lsp_types::DocumentDiagnosticReportResult::Report(lsp_types::DocumentDiagnosticReport::Full(
        lsp_types::RelatedFullDocumentDiagnosticReport {
            related_documents: None,
            full_document_diagnostic_report: lsp_types::FullDocumentDiagnosticReport {
                result_id: Some("aptos-analyzer".to_owned()),
                items: vec![],
            },
        },
    ))
}

pub(crate) fn handle_selection_range(
    snap: GlobalStateSnapshot,
    params: lsp_types::SelectionRangeParams,
) -> anyhow::Result<Option<Vec<lsp_types::SelectionRange>>> {
    let _p = tracing::info_span!("handle_selection_range").entered();
    let file_id = from_proto::file_id(&snap, &params.text_document.uri)?;
    let line_index = snap.file_line_index(file_id)?;
    let res: anyhow::Result<Vec<lsp_types::SelectionRange>> = params
        .positions
        .into_iter()
        .map(|position| {
            let offset = from_proto::offset(&line_index, position)?;
            let mut ranges = Vec::new();
            {
                let mut range = TextRange::new(offset, offset);
                loop {
                    ranges.push(range);
                    let frange = FileRange { file_id, range };
                    let next = snap.analysis.extend_selection(frange)?;
                    if next == range {
                        break;
                    } else {
                        range = next
                    }
                }
            }
            let mut range = lsp_types::SelectionRange {
                range: to_proto::range(&line_index, *ranges.last().unwrap()),
                parent: None,
            };
            for &r in ranges.iter().rev().skip(1) {
                range = lsp_types::SelectionRange {
                    range: to_proto::range(&line_index, r),
                    parent: Some(Box::new(range)),
                }
            }
            Ok(range)
        })
        .collect();

    Ok(Some(res?))
}

pub(crate) fn handle_hover(
    snap: GlobalStateSnapshot,
    params: lsp_types::HoverParams,
) -> anyhow::Result<Option<lsp_types::Hover>> {
    let _p = tracing::info_span!("handle_hover").entered();

    let file_position = from_proto::file_position(&snap, params.text_document_position_params)?;
    let info = match snap.analysis.hover(file_position)? {
        None => return Ok(None),
        Some(info) => info,
    };

    let line_index = snap.file_line_index(file_position.file_id)?;
    let range = to_proto::range(&line_index, info.range);
    let hover = lsp_types::Hover {
        contents: HoverContents::Markup(to_proto::markup_content(info.info.doc_string)),
        range: Some(range),
    };

    Ok(Some(hover))
}

pub(crate) fn handle_view_syntax_tree(
    snap: GlobalStateSnapshot,
    params: lsp_ext::ViewSyntaxTreeParams,
) -> anyhow::Result<String> {
    let _p = tracing::info_span!("handle_view_syntax_tree").entered();
    let file_id = from_proto::file_id(&snap, &params.text_document.uri)?;
    let syn = snap.analysis.view_syntax_tree(file_id)?;
    Ok(syn)
}

pub(crate) fn handle_formatting(
    snap: GlobalStateSnapshot,
    params: lsp_types::DocumentFormattingParams,
) -> anyhow::Result<Option<Vec<lsp_types::TextEdit>>> {
    let _p = tracing::info_span!("handle_formatting").entered();

    run_movefmt(&snap, params.text_document)
}

pub(crate) fn handle_analyzer_status(
    snap: GlobalStateSnapshot,
    params: lsp_ext::AnalyzerStatusParams,
) -> anyhow::Result<String> {
    let _p = tracing::info_span!("handle_analyzer_status").entered();

    let mut buf = String::new();

    // let mut file_id = None;
    // if let Some(tdi) = params.text_document {
    //     match from_proto::file_id(&snap, &tdi.uri) {
    //         Ok(it) => file_id = Some(it),
    //         Err(_) => format_to!(buf, "file {} not found in vfs", tdi.uri),
    //     }
    // }

    if snap.main_packages.is_empty() {
        buf.push_str("No packages\n")
    } else {
        buf.push_str("Packages:\n");
        format_to!(buf, "Loaded {:?} packages.\n", snap.main_packages.len(),);

        format_to!(
            buf,
            "Package root folders: {:?}",
            snap.main_packages
                .iter()
                .map(|ws| ws.content_root())
                .collect::<Vec<_>>()
        );
    }
    // buf.push_str("\nAnalysis:\n");
    // buf.push_str(
    //     &snap
    //         .analysis
    //         .status(file_id)
    //         .unwrap_or_else(|_| "Analysis retrieval was cancelled".to_owned()),
    // );

    buf.push_str("\nVersion: \n");
    format_to!(buf, "{}", crate::version());

    buf.push_str("\nConfiguration: \n");
    format_to!(buf, "{:#?}", snap.config);

    Ok(buf)
}

pub(crate) fn handle_inlay_hints(
    snap: GlobalStateSnapshot,
    params: InlayHintParams,
) -> anyhow::Result<Option<Vec<InlayHint>>> {
    let _p = tracing::info_span!("handle_inlay_hints").entered();
    let document_uri = &params.text_document.uri;
    let FileRange { file_id, range } = unwrap_or_return_default!(from_proto::file_range(
        &snap,
        &TextDocumentIdentifier::new(document_uri.to_owned()),
        params.range,
    )?);
    let line_index = snap.file_line_index(file_id)?;
    let range = TextRange::new(
        range.start().min(line_index.index.len()),
        range.end().min(line_index.index.len()),
    );

    let inlay_hints_config = snap.config.inlay_hints();
    Ok(Some(
        snap.analysis
            .inlay_hints(&inlay_hints_config, file_id, Some(range))?
            .into_iter()
            .map(|it| {
                to_proto::inlay_hint(
                    &snap,
                    &inlay_hints_config.fields_to_resolve,
                    &line_index,
                    file_id,
                    it,
                )
            })
            .collect::<Cancellable<Vec<_>>>()?,
    ))
}

pub(crate) fn handle_code_action(
    snap: GlobalStateSnapshot,
    params: lsp_types::CodeActionParams,
) -> anyhow::Result<Option<Vec<lsp_ext::CodeAction>>> {
    let _p = tracing::info_span!("handle_code_action").entered();

    if !snap.config.code_action_literals() {
        // We intentionally don't support command-based actions, as those either
        // require either custom client-code or server-initiated edits. Server
        // initiated edits break causality, so we avoid those.
        return Ok(None);
    }

    let file_id = from_proto::file_id(&snap, &params.text_document.uri)?;
    let frange = unwrap_or_return_default!(from_proto::file_range(
        &snap,
        &params.text_document,
        params.range
    )?);

    let mut assists_config = snap.config.assist();
    assists_config.allowed = params
        .context
        .only
        .clone()
        .map(|it| it.into_iter().filter_map(from_proto::assist_kind).collect());

    let mut res: Vec<lsp_ext::CodeAction> = Vec::new();

    let code_action_resolve_cap = snap.config.code_action_resolve();
    let resolve = if code_action_resolve_cap {
        AssistResolveStrategy::None
    } else {
        AssistResolveStrategy::All
    };
    let assists = snap.analysis.assists_with_fixes(
        &assists_config,
        &snap.config.diagnostics_config(),
        resolve,
        frange,
    )?;
    for (index, assist) in assists.into_iter().enumerate() {
        let resolve_data = if code_action_resolve_cap {
            Some((index, params.clone(), snap.file_version(file_id)))
        } else {
            None
        };
        let code_action = to_proto::code_action(&snap, assist, resolve_data)?;

        // Check if the client supports the necessary `ResourceOperation`s.
        let changes = code_action
            .edit
            .as_ref()
            .and_then(|it| it.document_changes.as_ref());
        if let Some(changes) = changes {
            for change in changes {
                if let lsp_ext::SnippetDocumentChangeOperation::Op(res_op) = change {
                    resource_ops_supported(&snap.config, resolve_resource_op(res_op))?
                }
            }
        }

        res.push(code_action)
    }

    // Fixes from `cargo check`.
    // for fix in snap
    //     .check_fixes
    //     .iter()
    //     .flat_map(|it| it.values())
    //     .filter_map(|it| it.get(&frange.file_id))
    //     .flatten()
    // {
    //     // FIXME: this mapping is awkward and shouldn't exist. Refactor
    //     // `snap.check_fixes` to not convert to LSP prematurely.
    //     let intersect_fix_range = fix
    //         .ranges
    //         .iter()
    //         .copied()
    //         .filter_map(|range| from_proto::text_range(&line_index, range).ok())
    //         .any(|fix_range| fix_range.intersect(frange.range).is_some());
    //     if intersect_fix_range {
    //         res.push(fix.action.clone());
    //     }
    // }

    Ok(Some(res))
}

pub(crate) fn handle_code_action_resolve(
    snap: GlobalStateSnapshot,
    mut code_action: lsp_ext::CodeAction,
) -> anyhow::Result<lsp_ext::CodeAction> {
    let _p = tracing::info_span!("handle_code_action_resolve").entered();
    let Some(params) = code_action.data.take() else {
        return Ok(code_action);
    };

    let file_id = from_proto::file_id(&snap, &params.code_action_params.text_document.uri)?;
    // .expect("we never provide code actions for excluded files");
    if snap.file_version(file_id) != params.version {
        return Err(invalid_params_error("stale code action".to_owned()).into());
    }
    let line_index = snap.file_line_index(file_id)?;
    let range = from_proto::text_range(&line_index, params.code_action_params.range)?;
    let frange = FileRange { file_id, range };
    // let source_root = snap.analysis.source_root_id(file_id)?;

    let mut assists_config = snap.config.assist(/*Some(source_root)*/);
    assists_config.allowed = params
        .code_action_params
        .context
        .only
        .map(|it| it.into_iter().filter_map(from_proto::assist_kind).collect());

    let (assist_index, assist_resolve) = match parse_action_id(&params.id) {
        Ok(parsed_data) => parsed_data,
        Err(e) => {
            return Err(invalid_params_error(format!(
                "Failed to parse action id string '{}': {e}",
                params.id
            ))
            .into());
        }
    };

    let expected_assist_id = assist_resolve.assist_id.clone();
    let expected_kind = assist_resolve.assist_kind;

    let assists = snap.analysis.assists_with_fixes(
        &assists_config,
        &snap.config.diagnostics_config(/*Some(source_root)*/),
        AssistResolveStrategy::Single(assist_resolve),
        frange,
    )?;

    let assist = match assists.get(assist_index) {
        Some(assist) => assist,
        None => return Err(invalid_params_error(format!(
            "Failed to find the assist for index {} provided by the resolve request. Resolve request assist id: {}",
            assist_index, params.id,
        ))
            .into())
    };
    if assist.id.0 != expected_assist_id || assist.id.1 != expected_kind {
        return Err(invalid_params_error(format!(
            "Mismatching assist at index {} for the resolve parameters given. Resolve request assist id: {}, actual id: {:?}.",
            assist_index, params.id, assist.id
        ))
            .into());
    }
    let ca = to_proto::code_action(&snap, assist.clone(), None)?;
    code_action.edit = ca.edit;
    code_action.command = ca.command;

    if let Some(edit) = code_action.edit.as_ref() {
        if let Some(changes) = edit.document_changes.as_ref() {
            for change in changes {
                if let lsp_ext::SnippetDocumentChangeOperation::Op(res_op) = change {
                    resource_ops_supported(&snap.config, resolve_resource_op(res_op))?
                }
            }
        }
    }

    Ok(code_action)
}

fn parse_action_id(action_id: &str) -> anyhow::Result<(usize, SingleResolve), String> {
    let id_parts = action_id.split(':').collect::<Vec<_>>();
    match id_parts.as_slice() {
        [assist_id_string, assist_kind_string, index_string, subtype_str] => {
            let assist_kind: AssistKind = assist_kind_string.parse()?;
            let index: usize = match index_string.parse() {
                Ok(index) => index,
                Err(e) => return Err(format!("Incorrect index string: {e}")),
            };
            let assist_subtype = subtype_str.parse::<usize>().ok();
            Ok((
                index,
                SingleResolve {
                    assist_id: assist_id_string.to_string(),
                    assist_kind,
                    assist_subtype,
                },
            ))
        }
        _ => Err("Action id contains incorrect number of segments".to_owned()),
    }
}

fn resource_ops_supported(config: &Config, kind: ResourceOperationKind) -> anyhow::Result<()> {
    if !matches!(config.workspace_edit_resource_operations(), Some(resops) if resops.contains(&kind)) {
        return Err(LspError::new(
            ErrorCode::RequestFailed as i32,
            format!(
                "Client does not support {} capability.",
                match kind {
                    ResourceOperationKind::Create => "create",
                    ResourceOperationKind::Rename => "rename",
                    ResourceOperationKind::Delete => "delete",
                }
            ),
        )
        .into());
    }

    Ok(())
}

fn resolve_resource_op(op: &ResourceOp) -> ResourceOperationKind {
    match op {
        ResourceOp::Create(_) => ResourceOperationKind::Create,
        ResourceOp::Rename(_) => ResourceOperationKind::Rename,
        ResourceOp::Delete(_) => ResourceOperationKind::Delete,
    }
}
