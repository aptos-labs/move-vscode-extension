use crate::diagnostics::to_proto_diagnostic;
use crate::global_state::GlobalStateSnapshot;
use crate::lsp::utils::invalid_params_error;
use crate::lsp::{LspError, from_proto, to_proto};
use crate::movefmt::run_movefmt;
use crate::{Config, lsp_ext, unwrap_or_return_default};
use ide::Cancellable;
use ide_db::assists::{AssistKind, AssistResolveStrategy, SingleResolve};
use ide_db::symbol_index::Query;
use line_index::TextRange;
use lsp_server::ErrorCode;
use lsp_types::{
    CodeActionOrCommand, DocumentHighlightKind, HoverContents, InlayHint, InlayHintParams, Location,
    PrepareRenameResponse, RenameParams, ResourceOp, ResourceOperationKind, SemanticTokensParams,
    SemanticTokensRangeParams, SemanticTokensRangeResult, SemanticTokensResult, TextDocumentIdentifier,
    WorkspaceEdit, WorkspaceSymbolParams,
};
use stdx::format_to;
use stdx::itertools::Itertools;
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

/// A value to use, when uncertain which limit to pick.
pub const DEFAULT_QUERY_SEARCH_LIMIT: usize = 100;

pub(crate) fn handle_workspace_symbol(
    snap: GlobalStateSnapshot,
    params: WorkspaceSymbolParams,
) -> anyhow::Result<Option<lsp_types::WorkspaceSymbolResponse>> {
    let _p = tracing::info_span!("handle_workspace_symbol").entered();

    let symbols = exec_query(&snap, Query::new(params.query), DEFAULT_QUERY_SEARCH_LIMIT)?;

    return Ok(Some(lsp_types::WorkspaceSymbolResponse::Nested(symbols)));

    fn exec_query(
        snap: &GlobalStateSnapshot,
        query: Query,
        limit: usize,
    ) -> anyhow::Result<Vec<lsp_types::WorkspaceSymbol>> {
        let mut res = Vec::new();
        for nav in snap.analysis.symbol_search(query, limit)? {
            let container_name = nav.container_name.as_ref().map(|v| v.to_string());

            let info = lsp_types::WorkspaceSymbol {
                name: match &nav.alias {
                    Some(alias) => format!("{} (alias for {})", alias, nav.name),
                    None => format!("{}", nav.name),
                },
                kind: nav
                    .kind
                    .map(to_proto::symbol_kind)
                    .unwrap_or(lsp_types::SymbolKind::VARIABLE),
                tags: None,
                container_name,
                location: lsp_types::OneOf::Left(to_proto::location_from_nav(snap, nav)?),
                data: None,
            };
            res.push(info);
        }
        Ok(res)
    }
}

pub(crate) fn handle_goto_definition(
    snap: GlobalStateSnapshot,
    params: lsp_types::GotoDefinitionParams,
) -> anyhow::Result<Option<lsp_types::GotoDefinitionResponse>> {
    let _p = tracing::info_span!("handle_goto_definition").entered();
    let position = from_proto::file_position(&snap, params.text_document_position_params)?;
    let nav_info = match snap.analysis.goto_definition_multi(position)? {
        None => return Ok(None),
        Some(it) => it,
    };
    let src = FileRange {
        file_id: position.file_id,
        range: nav_info.range,
    };
    let res = to_proto::goto_definition_response(&snap, Some(src), nav_info.info)?;
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

    let completion_config = &snap.config.completion();
    // FIXME: We should fix up the position when retrying the cancelled request instead
    position.offset = position.offset.min(line_index.index.len());
    let items =
        match snap
            .analysis
            .completions(completion_config, position, completion_trigger_character)?
        {
            None => return Ok(None),
            Some(items) => items,
        };

    let items = to_proto::completion_items(
        &snap.config,
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
    let _p = tracing::info_span!("handle_document_diagnostics").entered();

    let file_id = from_proto::file_id(&snap, &params.text_document.uri)?;

    let mut config = snap.config.diagnostics_config();
    if !config.enabled {
        return Ok(empty_diagnostic_report());
    }

    if !snap.analysis.is_local_package(file_id)? {
        return Ok(empty_diagnostic_report());
    }
    let package_metadata = snap.analysis.package_metadata(file_id)?;
    if package_metadata.is_none_or(|it| !it.resolve_deps) {
        config = config.for_assists();
        tracing::info!("only show assist diagnostics because of `resolve_deps = false`");
    }

    let line_index = snap.file_line_index(file_id)?;
    let diagnostics = snap
        .analysis
        .full_diagnostics(&config, AssistResolveStrategy::None, file_id)?
        .into_iter()
        .filter_map(|d| {
            let file = d.range.file_id;
            if file == file_id {
                let diagnostic = to_proto_diagnostic(&line_index, d);
                return Some(diagnostic);
            }
            None
        });
    Ok(lsp_types::DocumentDiagnosticReportResult::Report(
        lsp_types::DocumentDiagnosticReport::Full(lsp_types::RelatedFullDocumentDiagnosticReport {
            full_document_diagnostic_report: lsp_types::FullDocumentDiagnosticReport {
                result_id: Some("aptos-analyzer".to_owned()),
                items: diagnostics.collect(),
            },
            related_documents: None,
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
                range: to_proto::lsp_range(&line_index, *ranges.last().unwrap()),
                parent: None,
            };
            for &r in ranges.iter().rev().skip(1) {
                range = lsp_types::SelectionRange {
                    range: to_proto::lsp_range(&line_index, r),
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
    let range = to_proto::lsp_range(&line_index, info.range);
    let hover = lsp_types::Hover {
        contents: HoverContents::Markup(to_proto::markup_content(info.info.doc_string)),
        range: Some(range),
    };

    Ok(Some(hover))
}

pub(crate) fn handle_prepare_rename(
    snap: GlobalStateSnapshot,
    params: lsp_types::TextDocumentPositionParams,
) -> anyhow::Result<Option<PrepareRenameResponse>> {
    let _p = tracing::info_span!("handle_prepare_rename").entered();

    let position = from_proto::file_position(&snap, params)?;

    let change = snap
        .analysis
        .prepare_rename(position)?
        .map_err(to_proto::rename_error)?;

    let line_index = snap.file_line_index(position.file_id)?;
    let range = to_proto::lsp_range(&line_index, change.range);
    Ok(Some(PrepareRenameResponse::Range(range)))
}

pub(crate) fn handle_rename(
    snap: GlobalStateSnapshot,
    params: RenameParams,
) -> anyhow::Result<Option<WorkspaceEdit>> {
    let _p = tracing::info_span!("handle_rename").entered();
    let position = from_proto::file_position(&snap, params.text_document_position)?;

    let mut change = snap
        .analysis
        .rename(position, &params.new_name)?
        .map_err(to_proto::rename_error)?;

    // this is kind of a hack to prevent double edits from happening when moving files
    // When a module gets renamed by renaming the mod declaration this causes the file to move
    // which in turn will trigger a WillRenameFiles request to the server for which we reply with a
    // a second identical set of renames, the client will then apply both edits causing incorrect edits
    // with this we only emit source_file_edits in the WillRenameFiles response which will do the rename instead
    // See https://github.com/microsoft/vscode-languageserver-node/issues/752 for more info
    if !change.file_system_edits.is_empty() && snap.config.will_rename() {
        change.source_file_edits.clear();
    }

    let workspace_edit = to_proto::workspace_edit(&snap, change)?;

    if let Some(lsp_types::DocumentChanges::Operations(ops)) = workspace_edit.document_changes.as_ref() {
        for op in ops {
            if let lsp_types::DocumentChangeOperation::Op(doc_change_op) = op {
                resource_ops_supported(&snap.config, resolve_resource_op(doc_change_op))?
            }
        }
    }

    Ok(Some(workspace_edit))
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

pub(crate) fn handle_references(
    snap: GlobalStateSnapshot,
    params: lsp_types::ReferenceParams,
) -> anyhow::Result<Option<Vec<Location>>> {
    let _p = tracing::info_span!("handle_references").entered();

    let position =
        unwrap_or_return_default!(from_proto::file_position(&snap, params.text_document_position).ok());
    let Some(refs) = snap.analysis.find_all_refs(position, None)? else {
        return Ok(None);
    };

    let include_declaration = params.context.include_declaration;

    let decl = if include_declaration {
        refs.declaration.map(|decl| FileRange {
            file_id: decl.file_id,
            range: decl.focus_or_full_range(),
        })
    } else {
        None
    };

    let locations = refs
        .references
        .into_iter()
        .flat_map(|(file_id, refs)| {
            refs.into_iter()
                .map(move |text_range| FileRange { file_id, range: text_range })
        })
        .chain(decl)
        .unique()
        .filter_map(|frange| to_proto::location(&snap, frange).ok())
        .collect();

    Ok(Some(locations))
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
    _params: lsp_ext::AnalyzerStatusParams,
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

    if snap.all_packages.is_empty() {
        buf.push_str("No packages\n")
    } else {
        buf.push_str("Packages:\n");
        format_to!(buf, "Loaded {:?} packages.\n", snap.all_packages.len(),);

        format_to!(
            buf,
            "Package root folders: {:?}",
            snap.all_packages
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

pub(crate) fn handle_document_highlight(
    snap: GlobalStateSnapshot,
    params: lsp_types::DocumentHighlightParams,
) -> anyhow::Result<Option<Vec<lsp_types::DocumentHighlight>>> {
    let _p = tracing::info_span!("handle_document_highlight").entered();

    let position = from_proto::file_position(&snap, params.text_document_position_params)?;
    let line_index = snap.file_line_index(position.file_id)?;
    // let package_id = snap.analysis.package_id(position.file_id)?;

    let refs = match snap.analysis.highlight_related(position)? {
        None => return Ok(None),
        Some(refs) => refs,
    };
    let res = refs
        .into_iter()
        .map(|range| lsp_types::DocumentHighlight {
            range: to_proto::lsp_range(&line_index, range),
            kind: Some(DocumentHighlightKind::TEXT),
        })
        .collect();
    Ok(Some(res))
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
) -> anyhow::Result<Option<lsp_types::CodeActionResponse>> {
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

    let mut res = vec![];

    let code_action_resolve_cap = snap.config.code_action_resolve();
    let resolve = if code_action_resolve_cap {
        AssistResolveStrategy::None
    } else {
        AssistResolveStrategy::All
    };
    let assists = snap.analysis.assists_with_fixes(
        &assists_config,
        &snap.config.diagnostics_config().for_assists(),
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
        if let Some(lsp_types::DocumentChanges::Operations(ops)) = changes {
            for change_op in ops {
                if let lsp_types::DocumentChangeOperation::Op(res_op) = change_op {
                    resource_ops_supported(&snap.config, resolve_resource_op(&res_op))?
                }
            }
        }
        res.push(CodeActionOrCommand::CodeAction(code_action));
    }

    Ok(Some(res))
}

pub(crate) fn handle_code_action_resolve(
    snap: GlobalStateSnapshot,
    mut code_action: lsp_types::CodeAction,
) -> anyhow::Result<lsp_types::CodeAction> {
    let _p = tracing::info_span!("handle_code_action_resolve").entered();
    let Some(params) = code_action
        .data
        .take()
        .and_then(|it| serde_json::from_value::<lsp_ext::CodeActionData>(it).ok())
    else {
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

    let mut assists_config = snap.config.assist();
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
        &snap.config.diagnostics_config().for_assists(),
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
        if let Some(lsp_types::DocumentChanges::Operations(ops)) = edit.document_changes.as_ref() {
            for change_op in ops {
                if let lsp_types::DocumentChangeOperation::Op(res_op) = change_op {
                    resource_ops_supported(&snap.config, resolve_resource_op(&res_op))?
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
