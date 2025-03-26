use crate::diagnostics::convert_diagnostic;
use crate::global_state::{FetchWorkspaceRequest, GlobalState, GlobalStateSnapshot};
use crate::lsp::{from_proto, to_proto};
use crate::try_default;
use lang::files::FileRange;
use line_index::TextRange;
use lsp_types::{HoverContents, Range, SemanticTokensParams, SemanticTokensResult};

pub(crate) fn handle_workspace_reload(state: &mut GlobalState, _: ()) -> anyhow::Result<()> {
    // state.proc_macro_clients = Arc::from_iter([]);
    // state.build_deps_changed = false;

    let req = FetchWorkspaceRequest {
        path: None,
        force_crate_graph_reload: false,
    };
    state
        .fetch_workspaces_queue
        .request_op("reload workspace request".to_owned(), req);
    Ok(())
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
    // // Avoid flashing a bunch of unresolved references when the proc-macro servers haven't been spawned yet.
    // highlight_config.syntactic_name_ref_highlighting =
    //     snap.workspaces.is_empty() || !snap.proc_macros_loaded;

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

    let completion_list = lsp_types::CompletionList {
        is_incomplete: true,
        items,
    };
    Ok(Some(completion_list.into()))
}

pub(crate) fn handle_document_diagnostics(
    snap: GlobalStateSnapshot,
    params: lsp_types::DocumentDiagnosticParams,
) -> anyhow::Result<lsp_types::DocumentDiagnosticReportResult> {
    let empty = || {
        lsp_types::DocumentDiagnosticReportResult::Report(lsp_types::DocumentDiagnosticReport::Full(
            lsp_types::RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: lsp_types::FullDocumentDiagnosticReport {
                    result_id: Some("rust-analyzer".to_owned()),
                    items: vec![],
                },
            },
        ))
    };

    let file_id = from_proto::file_id(&snap, &params.text_document.uri)?;
    // let source_root = snap.analysis.source_root_id(file_id)?;
    // if !snap.analysis.is_local_source_root(source_root)? {
    //     return Ok(empty());
    // }
    let config = snap.config.diagnostics(/*Some(source_root)*/);
    if !config.enabled {
        return Ok(empty());
    }
    let line_index = snap.file_line_index(file_id)?;
    // let supports_related = false;
    // let supports_related = snap.config.text_document_diagnostic_related_document_support();

    // let mut related_documents = FxHashMap::default();
    let diagnostics = snap
        .analysis
        .syntax_diagnostics(&config, file_id)?
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
