use crate::global_state::GlobalStateSnapshot;
use crate::line_index::{LineEndings, LineIndex, PositionEncoding};
use crate::lsp::utils::invalid_params_error;
use crate::lsp::{LspError, semantic_tokens};
use crate::{Config, lsp_ext};
use camino::{Utf8Component, Utf8Prefix};
use ide::inlay_hints::{
    InlayFieldsToResolve, InlayHint, InlayHintLabel, InlayHintLabelPart, InlayHintPosition, InlayKind,
    InlayTooltip, LazyProperty,
};
use ide::syntax_highlighting::tags::{Highlight, HlOperator, HlPunct, HlTag};
use ide::{Cancellable, HlRange, NavigationTarget};
use ide_completion::item::{CompletionItem, CompletionItemKind};
use ide_db::assists::{Assist, AssistKind};
use ide_db::rename::RenameError;
use ide_db::source_change::{FileSystemEdit, SourceChange};
use ide_db::text_edit::{TextChange, TextEdit};
use ide_db::{Severity, SymbolKind};
use line_index::{TextRange, TextSize};
use lsp_types::{DocumentChanges, OneOf};
use std::hash::Hasher;
use std::mem;
use std::ops::Not;
use std::sync::atomic::{AtomicU32, Ordering};
use stdx::itertools::Itertools;
use syntax::files::FileRange;
use vfs::{AbsPath, FileId};

pub(crate) fn lsp_position(line_index: &LineIndex, offset: TextSize) -> lsp_types::Position {
    let line_col = line_index.index.line_col(offset);
    match line_index.encoding {
        PositionEncoding::Utf8 => lsp_types::Position::new(line_col.line, line_col.col),
        PositionEncoding::Wide(enc) => {
            let line_col = line_index.index.to_wide(enc, line_col).unwrap();
            lsp_types::Position::new(line_col.line, line_col.col)
        }
    }
}

pub(crate) fn lsp_range(line_index: &LineIndex, range: TextRange) -> lsp_types::Range {
    let start = lsp_position(line_index, range.start());
    let end = lsp_position(line_index, range.end());
    lsp_types::Range::new(start, end)
}

pub(crate) fn symbol_kind(symbol_kind: SymbolKind) -> lsp_types::SymbolKind {
    match symbol_kind {
        SymbolKind::Function => lsp_types::SymbolKind::FUNCTION,
        SymbolKind::Method => lsp_types::SymbolKind::METHOD,
        SymbolKind::Struct => lsp_types::SymbolKind::STRUCT,
        SymbolKind::Enum => lsp_types::SymbolKind::ENUM,
        SymbolKind::EnumVariant => lsp_types::SymbolKind::ENUM_MEMBER,
        SymbolKind::Attribute => lsp_types::SymbolKind::FUNCTION,
        SymbolKind::Module => lsp_types::SymbolKind::MODULE,
        SymbolKind::TypeParam => lsp_types::SymbolKind::TYPE_PARAMETER,
        SymbolKind::Field => lsp_types::SymbolKind::FIELD,
        SymbolKind::Const => lsp_types::SymbolKind::CONSTANT,
        SymbolKind::Local
        | SymbolKind::ValueParam
        | SymbolKind::Label
        | SymbolKind::GlobalVariableDecl => lsp_types::SymbolKind::VARIABLE,
        SymbolKind::Vector => lsp_types::SymbolKind::FUNCTION,
        SymbolKind::Assert => lsp_types::SymbolKind::FUNCTION,
    }
}

pub(crate) fn diagnostic_severity(severity: Severity) -> lsp_types::DiagnosticSeverity {
    match severity {
        Severity::Error => lsp_types::DiagnosticSeverity::ERROR,
        Severity::Warning => lsp_types::DiagnosticSeverity::WARNING,
        Severity::WeakWarning => lsp_types::DiagnosticSeverity::INFORMATION,
        // unreachable
        Severity::Allow => lsp_types::DiagnosticSeverity::INFORMATION,
    }
}

pub(crate) fn completion_item_kind(
    completion_item_kind: CompletionItemKind,
) -> lsp_types::CompletionItemKind {
    match completion_item_kind {
        CompletionItemKind::Binding => lsp_types::CompletionItemKind::VARIABLE,
        CompletionItemKind::BuiltinType => lsp_types::CompletionItemKind::STRUCT,
        CompletionItemKind::Keyword => lsp_types::CompletionItemKind::KEYWORD,
        CompletionItemKind::Expression => lsp_types::CompletionItemKind::SNIPPET,
        CompletionItemKind::UnresolvedReference => lsp_types::CompletionItemKind::REFERENCE,
        CompletionItemKind::SymbolKind(symbol) => match symbol {
            SymbolKind::Attribute => lsp_types::CompletionItemKind::FUNCTION,
            SymbolKind::Method => lsp_types::CompletionItemKind::METHOD,
            SymbolKind::Const => lsp_types::CompletionItemKind::CONSTANT,
            SymbolKind::Enum => lsp_types::CompletionItemKind::ENUM,
            SymbolKind::Field => lsp_types::CompletionItemKind::FIELD,
            SymbolKind::Function => lsp_types::CompletionItemKind::FUNCTION,
            SymbolKind::Label => lsp_types::CompletionItemKind::VARIABLE,
            SymbolKind::Local => lsp_types::CompletionItemKind::VARIABLE,
            SymbolKind::Module => lsp_types::CompletionItemKind::MODULE,
            SymbolKind::Struct => lsp_types::CompletionItemKind::STRUCT,
            SymbolKind::TypeParam => lsp_types::CompletionItemKind::TYPE_PARAMETER,
            SymbolKind::ValueParam => lsp_types::CompletionItemKind::VALUE,
            SymbolKind::EnumVariant => lsp_types::CompletionItemKind::ENUM_MEMBER,
            SymbolKind::GlobalVariableDecl => lsp_types::CompletionItemKind::VARIABLE,
            SymbolKind::Vector => lsp_types::CompletionItemKind::FUNCTION,
            SymbolKind::Assert => lsp_types::CompletionItemKind::FUNCTION,
        },
    }
}

pub(crate) fn lsp_text_edit(line_index: &LineIndex, change: TextChange) -> lsp_types::TextEdit {
    let range = lsp_range(line_index, change.range);
    let new_text = match line_index.endings {
        LineEndings::Unix => change.new_text,
        LineEndings::Dos => change.new_text.replace('\n', "\r\n"),
    };
    lsp_types::TextEdit { range, new_text }
}

pub(crate) fn lsp_completion_text_edit(
    line_index: &LineIndex,
    insert_replace_at: Option<lsp_types::Position>,
    text_change: TextChange,
) -> lsp_types::CompletionTextEdit {
    let text_edit = lsp_text_edit(line_index, text_change);
    match insert_replace_at {
        Some(cursor_pos) => lsp_types::InsertReplaceEdit {
            new_text: text_edit.new_text,
            insert: lsp_types::Range {
                start: text_edit.range.start,
                end: cursor_pos,
            },
            replace: text_edit.range,
        }
        .into(),
        None => text_edit.into(),
    }
}

pub(crate) fn text_edit_vec(line_index: &LineIndex, text_edit: TextEdit) -> Vec<lsp_types::TextEdit> {
    text_edit
        .into_iter()
        .map(|indel| lsp_text_edit(line_index, indel))
        .collect()
}

/// Fails if invoked on in-memory FileId, i.e. on builtins.
pub(crate) fn url(snap: &GlobalStateSnapshot, file_id: FileId) -> lsp_types::Url {
    snap.file_id_to_url(file_id)
}

/// Returns a `Url` object from a given path, will lowercase drive letters if present.
/// This will only happen when processing windows paths.
///
/// When processing non-windows path, this is essentially the same as `Url::from_file_path`.
pub(crate) fn url_from_abs_path(path: &AbsPath) -> lsp_types::Url {
    let url = lsp_types::Url::from_file_path(path).unwrap();
    match path.components().next() {
        Some(Utf8Component::Prefix(prefix))
            if matches!(prefix.kind(), Utf8Prefix::Disk(_) | Utf8Prefix::VerbatimDisk(_)) =>
        {
            // Need to lowercase driver letter
        }
        _ => return url,
    }

    let driver_letter_range = {
        let (scheme, drive_letter, _rest) = match url.as_str().splitn(3, ':').collect_tuple() {
            Some(it) => it,
            None => return url,
        };
        let start = scheme.len() + ':'.len_utf8();
        start..(start + drive_letter.len())
    };

    // Note: lowercasing the `path` itself doesn't help, the `Url::parse`
    // machinery *also* canonicalizes the drive letter. So, just massage the
    // string in place.
    let mut url: String = url.into();
    url[driver_letter_range].make_ascii_lowercase();
    lsp_types::Url::parse(&url).unwrap()
}

pub(crate) fn goto_definition_response(
    snap: &GlobalStateSnapshot,
    src: Option<FileRange>,
    targets: Vec<NavigationTarget>,
) -> Cancellable<lsp_types::GotoDefinitionResponse> {
    if snap.config.location_link() {
        let links = targets
            .into_iter()
            .unique_by(|nav| (nav.file_id, nav.full_range, nav.focus_range))
            .map(|nav| location_link(snap, src, nav))
            .collect::<Cancellable<Vec<_>>>()?;
        Ok(links.into())
    } else {
        let locations = targets
            .into_iter()
            .map(|nav| FileRange {
                file_id: nav.file_id,
                range: nav.focus_or_full_range(),
            })
            .unique()
            .map(|range| location(snap, range))
            .collect::<Cancellable<Vec<_>>>()?;
        Ok(locations.into())
    }
}

pub(crate) fn location_link(
    snap: &GlobalStateSnapshot,
    src: Option<FileRange>,
    target: NavigationTarget,
) -> Cancellable<lsp_types::LocationLink> {
    let origin_selection_range = match src {
        Some(src) => {
            let line_index = snap.file_line_index(src.file_id)?;
            let range = lsp_range(&line_index, src.range);
            Some(range)
        }
        None => None,
    };
    let (target_uri, target_range, target_selection_range) = location_info(snap, target)?;
    let res = lsp_types::LocationLink {
        origin_selection_range,
        target_uri,
        target_range,
        target_selection_range,
    };
    Ok(res)
}

fn location_info(
    snap: &GlobalStateSnapshot,
    target: NavigationTarget,
) -> Cancellable<(lsp_types::Url, lsp_types::Range, lsp_types::Range)> {
    let line_index = snap.file_line_index(target.file_id)?;

    let target_uri = url(snap, target.file_id);
    let target_range = lsp_range(&line_index, target.full_range);
    let target_selection_range = target
        .focus_range
        .map(|it| lsp_range(&line_index, it))
        .unwrap_or(target_range);
    Ok((target_uri, target_range, target_selection_range))
}

pub(crate) fn optional_versioned_text_document_identifier(
    snap: &GlobalStateSnapshot,
    file_id: FileId,
) -> lsp_types::OptionalVersionedTextDocumentIdentifier {
    let url = url(snap, file_id);
    let version = snap.url_file_version(&url);
    lsp_types::OptionalVersionedTextDocumentIdentifier { uri: url, version }
}

pub(crate) fn location(
    snap: &GlobalStateSnapshot,
    frange: FileRange,
) -> Cancellable<lsp_types::Location> {
    let url = url(snap, frange.file_id);
    let line_index = snap.file_line_index(frange.file_id)?;
    let range = lsp_range(&line_index, frange.range);
    let loc = lsp_types::Location::new(url, range);
    Ok(loc)
}

static TOKEN_RESULT_COUNTER: AtomicU32 = AtomicU32::new(1);

pub(crate) fn semantic_tokens(
    text: &str,
    line_index: &LineIndex,
    highlights: Vec<HlRange>,
    // semantics_tokens_augments_syntax_tokens: bool,
    // non_standard_tokens: bool,
) -> lsp_types::SemanticTokens {
    let id = TOKEN_RESULT_COUNTER.fetch_add(1, Ordering::SeqCst).to_string();
    let mut builder = semantic_tokens::SemanticTokensBuilder::new(id);

    for highlight_range in highlights {
        if highlight_range.highlight.is_empty() {
            continue;
        }

        // if semantics_tokens_augments_syntax_tokens {
        //     match highlight_range.highlight.tag {
        //         HlTag::BoolLiteral
        //         | HlTag::ByteLiteral
        //         | HlTag::CharLiteral
        //         | HlTag::Comment
        //         | HlTag::Keyword
        //         | HlTag::NumericLiteral
        //         | HlTag::Operator(_)
        //         | HlTag::Punctuation(_)
        //         | HlTag::StringLiteral
        //         | HlTag::None
        //         if highlight_range.highlight.mods.is_empty() =>
        //             {
        //                 continue
        //             }
        //         _ => (),
        //     }
        // }

        let ty = semantic_token_type(highlight_range.highlight);

        // if !non_standard_tokens {
        //     ty = match standard_fallback_type(ty) {
        //         Some(ty) => ty,
        //         None => continue,
        //     };
        //     mods.standard_fallback();
        // }
        let token_index = semantic_tokens::type_index(ty);

        for mut text_range in line_index.index.lines(highlight_range.range) {
            if text[text_range].ends_with('\n') {
                text_range = TextRange::new(text_range.start(), text_range.end() - TextSize::of('\n'));
            }
            let range = lsp_range(line_index, text_range);
            builder.push(range, token_index);
        }
    }

    builder.build()
}

fn semantic_token_type(highlight: Highlight) -> lsp_types::SemanticTokenType {
    use semantic_tokens::types;

    match highlight.tag {
        HlTag::Symbol(symbol) => match symbol {
            SymbolKind::Attribute => types::DECORATOR,
            SymbolKind::Module => types::NAMESPACE,
            SymbolKind::Field => types::PROPERTY,
            SymbolKind::TypeParam => types::TYPE_PARAMETER,
            SymbolKind::Label => types::LABEL,
            SymbolKind::ValueParam => types::PARAMETER,
            SymbolKind::Local => types::VARIABLE,
            SymbolKind::Method => types::METHOD,
            SymbolKind::Function => types::FUNCTION,
            SymbolKind::Const => types::CONST,
            SymbolKind::Struct => types::STRUCT,
            SymbolKind::Enum => types::ENUM,
            SymbolKind::EnumVariant => types::ENUM_MEMBER,
            SymbolKind::GlobalVariableDecl => types::VARIABLE,
            SymbolKind::Vector => types::MACRO,
            SymbolKind::Assert => types::MACRO,
        },
        HlTag::AttributeBracket => types::ATTRIBUTE_BRACKET,
        HlTag::BoolLiteral => types::BOOLEAN,
        HlTag::BuiltinType => types::BUILTIN_TYPE,
        HlTag::NumericLiteral => types::NUMBER,
        HlTag::Comment => types::COMMENT,
        HlTag::Keyword => types::KEYWORD,
        HlTag::None => types::GENERIC,
        HlTag::Operator(op) => match op {
            HlOperator::Bitwise => types::BITWISE,
            HlOperator::Arithmetic => types::ARITHMETIC,
            HlOperator::Logical => types::LOGICAL,
            HlOperator::Comparison => types::COMPARISON,
            HlOperator::Other => types::OPERATOR,
        },
        HlTag::StringLiteral => types::STRING,
        HlTag::UnresolvedReference => types::UNRESOLVED_REFERENCE,
        HlTag::Punctuation(punct) => match punct {
            HlPunct::Bracket => types::BRACKET,
            HlPunct::Brace => types::BRACE,
            HlPunct::Parenthesis => types::PARENTHESIS,
            HlPunct::Angle => types::ANGLE,
            HlPunct::Comma => types::COMMA,
            HlPunct::Dot => types::DOT,
            HlPunct::Colon => types::COLON,
            HlPunct::Semi => types::SEMICOLON,
            HlPunct::Other => types::PUNCTUATION,
            HlPunct::MacroBang => types::MACRO_BANG,
        },
    }
}

pub(crate) fn completion_items(
    config: &Config,
    line_index: &LineIndex,
    version: Option<i32>,
    params: lsp_types::TextDocumentPositionParams,
    completion_trigger_character: Option<char>,
    items: Vec<CompletionItem>,
) -> Vec<lsp_types::CompletionItem> {
    let mut res = Vec::with_capacity(items.len());
    for item in items {
        completion_item(&mut res, config, line_index, &params, item);
    }
    res
}

fn completion_item(
    acc: &mut Vec<lsp_types::CompletionItem>,
    config: &Config,
    line_index: &LineIndex,
    params: &lsp_types::TextDocumentPositionParams,
    item: CompletionItem,
) {
    let filter_text = item.lookup().to_owned();
    let detail = item.detail.clone();
    // let documentation = item.documentation.clone().map(documentation);

    // LSP does not allow arbitrary edits in completion, so we have to do a
    // non-trivial mapping here.
    let insert_replace_at = config.insert_replace_support().then_some(params.position);
    let mut text_edit = None;
    let mut additional_text_edits = Vec::new();
    let ident_range = item.source_range;
    for text_change in &item.text_edit {
        // if text change does not affect the ident itself, put it into the `additional_text_edits`
        if !text_change.range.contains_range(ident_range) {
            assert!(ident_range.intersect(text_change.range).is_none());
            let text_edit = lsp_text_edit(line_index, text_change.clone());
            additional_text_edits.push(text_edit);
            continue;
        }

        if text_change.range == ident_range {
            let edit = lsp_completion_text_edit(line_index, insert_replace_at, text_change.clone());
            text_edit = Some(edit);
            continue;
        }

        assert_eq!(ident_range.end(), text_change.range.end());
        let range1 = TextRange::new(text_change.range.start(), ident_range.start());
        let indel1 = TextChange::delete(range1);
        let indel2 = TextChange::replace(ident_range, text_change.new_text.clone());
        additional_text_edits.push(lsp_text_edit(line_index, indel1));
        text_edit = Some(lsp_completion_text_edit(line_index, insert_replace_at, indel2));
    }
    let insert_text_format = item.is_snippet.then_some(lsp_types::InsertTextFormat::SNIPPET);

    let mut lsp_item = lsp_types::CompletionItem {
        label: item.label.primary.to_string(),
        detail,
        filter_text: Some(filter_text),
        kind: Some(completion_item_kind(item.kind)),
        text_edit,
        additional_text_edits: additional_text_edits
            .is_empty()
            .not()
            .then_some(additional_text_edits),
        // documentation,
        insert_text_format,
        ..Default::default()
    };

    if config.completion_label_details_support() {
        let has_label_details = item.label.detail_left.is_some() || item.label.detail_right.is_some();
        if has_label_details {
            lsp_item.label_details = Some(lsp_types::CompletionItemLabelDetails {
                detail: item.label.detail_left.clone(),
                description: item.label.detail_right.clone(),
            });
        }
    } else if let Some(label_detail) = &item.label.detail_left {
        lsp_item.label.push_str(label_detail.as_str());
    }

    acc.push(lsp_item);
}

pub(crate) fn markup_content(markup: String) -> lsp_types::MarkupContent {
    // todo: format docs later
    lsp_types::MarkupContent {
        kind: lsp_types::MarkupKind::Markdown,
        value: markup,
    }
}

pub(crate) fn text_document_edit(
    snap: &GlobalStateSnapshot,
    file_id: FileId,
    edit: TextEdit,
) -> Cancellable<lsp_types::TextDocumentEdit> {
    let text_document = optional_versioned_text_document_identifier(snap, file_id);
    let line_index = snap.file_line_index(file_id)?;
    // let client_supports_annotations = snap.config.change_annotation_support();
    let edits = {
        // let annotation = edit.change_annotation();
        edit.into_iter()
            .map(|it| OneOf::Left(lsp_text_edit(&line_index, it)))
            .collect::<Vec<_>>()
    };

    // if snap.analysis.is_library_file(file_id)? && snap.config.change_annotation_support() {
    //     for edit in &mut edits {
    //         edit.annotation_id = Some(outside_workspace_annotation_id())
    //     }
    // }

    Ok(lsp_types::TextDocumentEdit { text_document, edits })
}

pub(crate) fn text_document_ops(
    snap: &GlobalStateSnapshot,
    file_system_edit: FileSystemEdit,
) -> Cancellable<Vec<lsp_types::DocumentChangeOperation>> {
    let mut ops = Vec::new();
    match file_system_edit {
        FileSystemEdit::CreateFile { dst, initial_contents } => {
            let uri = snap.anchored_path(&dst);
            let create_file = lsp_types::ResourceOp::Create(lsp_types::CreateFile {
                uri: uri.clone(),
                options: None,
                annotation_id: None,
            });
            ops.push(lsp_types::DocumentChangeOperation::Op(create_file));
            if !initial_contents.is_empty() {
                let text_document =
                    lsp_types::OptionalVersionedTextDocumentIdentifier { uri, version: None };
                let text_edit = lsp_types::TextEdit {
                    range: lsp_types::Range::default(),
                    new_text: initial_contents,
                    // insert_text_format: Some(lsp_types::InsertTextFormat::PLAIN_TEXT),
                    // annotation_id: None,
                };
                let edit_file = lsp_types::TextDocumentEdit {
                    text_document,
                    edits: vec![OneOf::Left(text_edit)],
                };
                ops.push(lsp_types::DocumentChangeOperation::Edit(edit_file));
            }
        }
        // FileSystemEdit::MoveFile { src, dst } => {
        //     let old_uri = snap.file_id_to_url(src);
        //     let new_uri = snap.anchored_path(&dst);
        //     let rename_file = lsp_types::RenameFile {
        //         old_uri,
        //         new_uri,
        //         options: None,
        //         annotation_id: None,
        //     };
        //     // if snap.analysis.is_library_file(src).ok() == Some(true)
        //     //     && snap.config.change_annotation_support()
        //     // {
        //     //     rename_file.annotation_id = Some(outside_workspace_annotation_id())
        //     // }
        //     ops.push(lsp_ext::SnippetDocumentChangeOperation::Op(
        //         lsp_types::ResourceOp::Rename(rename_file),
        //     ))
        // }
        // FileSystemEdit::MoveDir { src, src_id, dst } => {
        //     let old_uri = snap.anchored_path(&src);
        //     let new_uri = snap.anchored_path(&dst);
        //     let rename_file = lsp_types::RenameFile {
        //         old_uri,
        //         new_uri,
        //         options: None,
        //         annotation_id: None,
        //     };
        //     // if snap.analysis.is_library_file(src_id).ok() == Some(true)
        //     //     && snap.config.change_annotation_support()
        //     // {
        //     //     rename_file.annotation_id = Some(outside_workspace_annotation_id())
        //     // }
        //     ops.push(lsp_ext::SnippetDocumentChangeOperation::Op(
        //         lsp_types::ResourceOp::Rename(rename_file),
        //     ))
        // }
        _ => (),
    }
    Ok(ops)
}

pub(crate) fn workspace_edit(
    snap: &GlobalStateSnapshot,
    mut source_change: SourceChange,
) -> Cancellable<lsp_types::WorkspaceEdit> {
    let mut document_changes: Vec<lsp_types::DocumentChangeOperation> = Vec::new();

    for op in &mut source_change.file_system_edits {
        if let FileSystemEdit::CreateFile { dst, initial_contents } = op {
            // replace with a placeholder to avoid cloneing the edit
            let op = FileSystemEdit::CreateFile {
                dst: dst.clone(),
                initial_contents: mem::take(initial_contents),
            };
            let ops = text_document_ops(snap, op)?;
            document_changes.extend_from_slice(&ops);
        }
    }
    for (file_id, edit) in source_change.source_file_edits {
        let edit = text_document_edit(snap, file_id, edit)?;
        document_changes.push(lsp_types::DocumentChangeOperation::Edit(edit));
    }
    // for op in source_change.file_system_edits {
    //     if !matches!(op, FileSystemEdit::CreateFile { .. }) {
    //         let ops = text_document_ops(snap, op)?;
    //         document_changes.extend_from_slice(&ops);
    //     }
    // }
    let workspace_edit = lsp_types::WorkspaceEdit {
        changes: None,
        document_changes: Some(DocumentChanges::Operations(document_changes)),
        change_annotations: None,
    };
    // if snap.config.change_annotation_support() {
    //     workspace_edit.change_annotations = Some(
    //         once((
    //             outside_workspace_annotation_id(),
    //             lsp_types::ChangeAnnotation {
    //                 label: String::from("Edit outside of the workspace"),
    //                 needs_confirmation: Some(true),
    //                 description: Some(String::from(
    //                     "This edit lies outside of the workspace and may affect dependencies",
    //                 )),
    //             },
    //         ))
    //             .chain(source_change.annotations.into_iter().map(|(id, annotation)| {
    //                 (
    //                     id.to_string(),
    //                     lsp_types::ChangeAnnotation {
    //                         label: annotation.label,
    //                         description: annotation.description,
    //                         needs_confirmation: Some(annotation.needs_confirmation),
    //                     },
    //                 )
    //             }))
    //             .collect(),
    //     )
    // }
    Ok(workspace_edit)
}

pub(crate) fn code_action_kind(kind: AssistKind) -> lsp_types::CodeActionKind {
    match kind {
        AssistKind::Generate => lsp_types::CodeActionKind::EMPTY,
        AssistKind::QuickFix => lsp_types::CodeActionKind::QUICKFIX,
        AssistKind::Refactor => lsp_types::CodeActionKind::REFACTOR,
        AssistKind::RefactorExtract => lsp_types::CodeActionKind::REFACTOR_EXTRACT,
        AssistKind::RefactorInline => lsp_types::CodeActionKind::REFACTOR_INLINE,
        AssistKind::RefactorRewrite => lsp_types::CodeActionKind::REFACTOR_REWRITE,
    }
}

pub(crate) fn code_action(
    snap: &GlobalStateSnapshot,
    assist: Assist,
    resolve_data: Option<(usize, lsp_types::CodeActionParams, Option<i32>)>,
) -> Cancellable<lsp_types::CodeAction> {
    let mut res = lsp_types::CodeAction {
        title: assist.label.to_string(),
        kind: Some(code_action_kind(assist.id.1)),
        diagnostics: None,
        edit: None,
        command: None,
        is_preferred: None,
        disabled: None,
        data: None,
    };
    match (assist.source_change, resolve_data) {
        (Some(it), _) => res.edit = Some(workspace_edit(snap, it)?),
        (None, Some((index, code_action_params, version))) => {
            let data = Some(lsp_ext::CodeActionData {
                id: format!(
                    "{}:{}:{index}:{}",
                    assist.id.0,
                    assist.id.1.name(),
                    assist.id.2.map(|x| x.to_string()).unwrap_or("".to_owned())
                ),
                code_action_params,
                version,
            });
            res.data = serde_json::to_value(data).ok();
        }
        (None, None) => {
            stdx::never!("assist should always be resolved if client can't do lazy resolving")
        }
    };
    Ok(res)
}

pub(crate) fn inlay_hint(
    snap: &GlobalStateSnapshot,
    fields_to_resolve: &InlayFieldsToResolve,
    line_index: &LineIndex,
    file_id: FileId,
    mut inlay_hint: InlayHint,
) -> Cancellable<lsp_types::InlayHint> {
    let hint_needs_resolve = |hint: &InlayHint| -> Option<TextRange> {
        hint.resolve_parent.filter(|_| {
            hint.text_edit.as_ref().is_some_and(LazyProperty::is_lazy)
                || hint.label.parts.iter().any(|part| {
                    part.linked_location.as_ref().is_some_and(LazyProperty::is_lazy)
                        || part.tooltip.as_ref().is_some_and(LazyProperty::is_lazy)
                })
        })
    };

    // let resolve_range_and_hash = hint_needs_resolve(&inlay_hint).map(|range| {
    //     (
    //         range,
    //         std::hash::BuildHasher::hash_one(
    //             &std::hash::BuildHasherDefault::<FxHasher>::default(),
    //             &inlay_hint,
    //         ),
    //     )
    // });

    let mut something_to_resolve = false;
    let text_edits = inlay_hint
        .text_edit
        .take()
        .and_then(|it| match it {
            LazyProperty::Computed(it) => Some(it),
            LazyProperty::Lazy => {
                // something_to_resolve |=
                //     resolve_range_and_hash.is_some() && fields_to_resolve.resolve_text_edits;
                // something_to_resolve |= snap
                //     .config
                //     .visual_studio_code_version()
                //     .is_none_or(|version| VersionReq::parse(">=1.86.0").unwrap().matches(version))
                //     && resolve_range_and_hash.is_some()
                //     && fields_to_resolve.resolve_text_edits;
                None
            }
        })
        .map(|it| text_edit_vec(line_index, it));
    let (label, tooltip) = inlay_hint_label(
        snap,
        fields_to_resolve,
        // &mut something_to_resolve,
        // resolve_range_and_hash.is_some(),
        inlay_hint.label,
    )?;

    // let data = match resolve_range_and_hash {
    //     Some((resolve_range, hash)) if something_to_resolve => Some(
    //         to_value(lsp_ext::InlayHintResolveData {
    //             file_id: file_id.index(),
    //             hash: hash.to_string(),
    //             version: snap.file_version(file_id),
    //             resolve_range: range(line_index, resolve_range),
    //         })
    //         .unwrap(),
    //     ),
    //     _ => None,
    // };

    Ok(lsp_types::InlayHint {
        position: match inlay_hint.position {
            InlayHintPosition::Before => lsp_position(line_index, inlay_hint.range.start()),
            InlayHintPosition::After => lsp_position(line_index, inlay_hint.range.end()),
        },
        padding_left: Some(inlay_hint.pad_left),
        padding_right: Some(inlay_hint.pad_right),
        kind: match inlay_hint.kind {
            InlayKind::Parameter | InlayKind::GenericParameter => {
                Some(lsp_types::InlayHintKind::PARAMETER)
            }
            InlayKind::Type | InlayKind::Chaining => Some(lsp_types::InlayHintKind::TYPE),
            _ => None,
        },
        text_edits,
        data: None,
        tooltip,
        label,
    })
}

fn inlay_hint_label(
    snap: &GlobalStateSnapshot,
    fields_to_resolve: &InlayFieldsToResolve,
    // something_to_resolve: &mut bool,
    // needs_resolve: bool,
    mut label: InlayHintLabel,
) -> Cancellable<(lsp_types::InlayHintLabel, Option<lsp_types::InlayHintTooltip>)> {
    let (label, tooltip) = match &*label.parts {
        [InlayHintLabelPart { linked_location: None, .. }] => {
            let InlayHintLabelPart { text, tooltip, .. } = label.parts.pop().unwrap();
            let tooltip = tooltip.and_then(|it| match it {
                LazyProperty::Computed(it) => Some(it),
                LazyProperty::Lazy => {
                    // *something_to_resolve |= needs_resolve && fields_to_resolve.resolve_hint_tooltip;
                    None
                }
            });
            let hint_tooltip = match tooltip {
                Some(InlayTooltip::String(s)) => Some(lsp_types::InlayHintTooltip::String(s)),
                Some(InlayTooltip::Markdown(s)) => Some(lsp_types::InlayHintTooltip::MarkupContent(
                    lsp_types::MarkupContent {
                        kind: lsp_types::MarkupKind::Markdown,
                        value: s,
                    },
                )),
                None => None,
            };
            (lsp_types::InlayHintLabel::String(text), hint_tooltip)
        }
        _ => {
            let parts = label
                .parts
                .into_iter()
                .map(|part| {
                    let tooltip = part.tooltip.and_then(|it| match it {
                        LazyProperty::Computed(it) => Some(it),
                        LazyProperty::Lazy => {
                            // *something_to_resolve |= fields_to_resolve.resolve_label_tooltip;
                            None
                        }
                    });
                    let tooltip = match tooltip {
                        Some(InlayTooltip::String(s)) => {
                            Some(lsp_types::InlayHintLabelPartTooltip::String(s))
                        }
                        Some(InlayTooltip::Markdown(s)) => {
                            Some(lsp_types::InlayHintLabelPartTooltip::MarkupContent(
                                lsp_types::MarkupContent {
                                    kind: lsp_types::MarkupKind::Markdown,
                                    value: s,
                                },
                            ))
                        }
                        None => None,
                    };
                    let location = part
                        .linked_location
                        .and_then(|it| match it {
                            LazyProperty::Computed(it) => Some(it),
                            LazyProperty::Lazy => {
                                // *something_to_resolve |= fields_to_resolve.resolve_label_location;
                                None
                            }
                        })
                        .map(|range| location(snap, range))
                        .transpose()?;
                    Ok(lsp_types::InlayHintLabelPart {
                        value: part.text,
                        tooltip,
                        location,
                        command: None,
                    })
                })
                .collect::<Cancellable<_>>()?;
            (lsp_types::InlayHintLabel::LabelParts(parts), None)
        }
    };
    Ok((label, tooltip))
}

pub(crate) fn rename_error(err: RenameError) -> LspError {
    // This is wrong, but we don't have a better alternative I suppose?
    // https://github.com/microsoft/language-server-protocol/issues/1341
    invalid_params_error(err.to_string())
}
