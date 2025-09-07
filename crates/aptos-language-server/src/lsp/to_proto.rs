// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::global_state::GlobalStateSnapshot;
use crate::line_index::{LineIndex, PositionEncoding};
use crate::lsp::utils::invalid_params_error;
use crate::lsp::{LspError, semantic_tokens};
use crate::{Config, lsp_ext};
use camino::{Utf8Component, Utf8Prefix};
use ide::annotations::{Annotation, AnnotationKind};
use ide::inlay_hints::{
    InlayFieldsToResolve, InlayHint, InlayHintLabel, InlayHintLabelPart, InlayHintPosition, InlayKind,
    InlayTooltip, LazyProperty,
};
use ide::runnables::{Runnable, RunnableKind};
use ide::syntax_highlighting::tags::{Highlight, HlOperator, HlPunct, HlTag};
use ide::{Cancellable, HlRange, NavigationTarget, SignatureHelp};
use ide_completion::item::{CompletionItem, CompletionItemKind, CompletionRelevance};
use ide_db::assists::{Assist, AssistKind};
use ide_db::rename::RenameError;
use ide_db::source_change::{FileSystemEdit, SourceChange};
use ide_db::text_edit::{TextChange, TextEdit};
use ide_db::{Severity, SymbolKind};
use line_index::{TextRange, TextSize};
use lsp_types::{DocumentChanges, OneOf};
use std::hash::{DefaultHasher, Hasher};
use std::mem;
use std::ops::Not;
use std::sync::atomic::{AtomicU32, Ordering};
use stdext::line_endings::LineEndings;
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
        SymbolKind::Schema => lsp_types::SymbolKind::STRUCT,
    }
}

pub(crate) fn diagnostic_severity(severity: Severity) -> lsp_types::DiagnosticSeverity {
    match severity {
        Severity::Error => lsp_types::DiagnosticSeverity::ERROR,
        Severity::Warning => lsp_types::DiagnosticSeverity::WARNING,
        Severity::WeakWarning => lsp_types::DiagnosticSeverity::INFORMATION,
        // unreachable
        Severity::Hint => lsp_types::DiagnosticSeverity::HINT,
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
            SymbolKind::Schema => lsp_types::CompletionItemKind::STRUCT,
        },
    }
}

pub(crate) fn lsp_text_edit(line_index: &LineIndex, change: TextChange) -> lsp_types::TextEdit {
    let range = lsp_range(line_index, change.range);
    lsp_types::TextEdit {
        range,
        new_text: line_index.endings.map(change.new_text),
    }
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
        .map(|change| lsp_text_edit(line_index, change))
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

/// Prefer using `location_link`, if the client has the cap.
pub(crate) fn location_from_nav(
    snap: &GlobalStateSnapshot,
    nav: NavigationTarget,
) -> Cancellable<lsp_types::Location> {
    let url = url(snap, nav.file_id);
    let line_index = snap.file_line_index(nav.file_id)?;
    let range = lsp_range(&line_index, nav.focus_or_full_range());
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
            SymbolKind::Schema => types::STRUCT,
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
    tdpp: lsp_types::TextDocumentPositionParams,
    completion_trigger_character: Option<char>,
    items: Vec<CompletionItem>,
) -> Vec<lsp_types::CompletionItem> {
    let mut res = Vec::with_capacity(items.len());
    let max_relevance = items
        .iter()
        .map(|it| it.relevance.score())
        .max()
        .unwrap_or_default();
    for item in items {
        completion_item(
            &mut res,
            config,
            line_index,
            &tdpp,
            max_relevance,
            completion_trigger_character,
            item,
        );
    }
    res
}

fn completion_item(
    acc: &mut Vec<lsp_types::CompletionItem>,
    config: &Config,
    line_index: &LineIndex,
    tdpp: &lsp_types::TextDocumentPositionParams,
    max_relevance: u32,
    completion_trigger_character: Option<char>,
    item: CompletionItem,
) {
    let filter_text = item.lookup().to_owned();
    let detail = item.detail.clone();
    // let documentation = item.documentation.clone().map(documentation);

    // LSP does not allow arbitrary edits in completion, so we have to do a
    // non-trivial mapping here.
    let insert_replace_at = config.insert_replace_support().then_some(tdpp.position);
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

    set_score(&mut lsp_item, max_relevance, item.relevance);

    let resolve_data = if config.completion().enable_imports_on_the_fly
        && let Some(import_path) = item.import_to_add
    {
        let resolve_data = lsp_ext::CompletionResolveData {
            position: tdpp.clone(),
            version: None,
            import: lsp_ext::CompletionImport {
                full_import_path: import_path,
            },
            trigger_character: completion_trigger_character,
            // for_ref: false,
            // hash: BASE64_STANDARD.encode(completion_item_hash(&item, false)),
        };
        Some(serde_json::to_value(resolve_data).unwrap())
    } else {
        None
    };

    lsp_item.data = resolve_data;

    acc.push(lsp_item);
}

fn set_score(res: &mut lsp_types::CompletionItem, max_relevance: u32, relevance: CompletionRelevance) {
    if relevance.is_relevant() && relevance.score() == max_relevance {
        res.preselect = Some(true);
    }
    // The relevance needs to be inverted to come up with a sort score
    // because the client will sort ascending.
    let sort_score = relevance.score() ^ 0xFF_FF_FF_FF;
    // Zero pad the string to ensure values can be properly sorted
    // by the client. Hex format is used because it is easier to
    // visually compare very large values, which the sort text
    // tends to be since it is the opposite of the score.
    res.sort_text = Some(format!("{sort_score:08x}"));
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
    let edits = {
        edit.into_iter()
            .map(|it| OneOf::Left(lsp_text_edit(&line_index, it)))
            .collect::<Vec<_>>()
    };

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
                };
                let edit_file = lsp_types::TextDocumentEdit {
                    text_document,
                    edits: vec![OneOf::Left(text_edit)],
                };
                ops.push(lsp_types::DocumentChangeOperation::Edit(edit_file));
            }
        }
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

    Ok(lsp_types::WorkspaceEdit {
        changes: None,
        document_changes: Some(DocumentChanges::Operations(document_changes)),
        change_annotations: None,
    })
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

pub(crate) fn signature_help(call_info: SignatureHelp, label_offsets: bool) -> lsp_types::SignatureHelp {
    let (label, parameters) = match label_offsets {
        false => {
            let params = call_info
                .parameter_labels()
                .map(|label| lsp_types::ParameterInformation {
                    label: lsp_types::ParameterLabel::Simple(label.to_owned()),
                    documentation: None,
                })
                .collect::<Vec<_>>();
            let label = call_info.parameter_labels().join(", ");
            (label, params)
        }
        true => {
            let mut params = Vec::new();
            let mut label = String::new();
            let mut first = true;
            for param in call_info.parameter_labels() {
                if !first {
                    label.push_str(", ");
                }
                first = false;
                let start = label.chars().count() as u32;
                label.push_str(param);
                let end = label.chars().count() as u32;
                params.push(lsp_types::ParameterInformation {
                    label: lsp_types::ParameterLabel::LabelOffsets([start, end]),
                    documentation: None,
                });
            }

            (label, params)
        }
    };

    let active_parameter = call_info.active_parameter.map(|it| it as u32);

    let signature = lsp_types::SignatureInformation {
        label,
        parameters: Some(parameters),
        active_parameter,
        documentation: None,
    };
    lsp_types::SignatureHelp {
        signatures: vec![signature],
        active_signature: Some(0),
        active_parameter,
    }
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

    let resolve_range_and_hash = hint_needs_resolve(&inlay_hint).map(|range| {
        (
            range,
            std::hash::BuildHasher::hash_one(
                &std::hash::BuildHasherDefault::<DefaultHasher>::default(),
                &inlay_hint,
            ),
        )
    });

    let mut something_to_resolve = false;

    let (label, tooltip) = inlay_hint_label(
        snap,
        fields_to_resolve,
        &mut something_to_resolve,
        resolve_range_and_hash.is_some(),
        inlay_hint.label,
    )?;

    let data = match resolve_range_and_hash {
        Some((resolve_range, hash)) if something_to_resolve => Some(
            serde_json::to_value(lsp_ext::InlayHintResolveData {
                file_id: file_id.index(),
                hash: hash.to_string(),
                version: snap.file_version(file_id),
                resolve_range: lsp_range(line_index, resolve_range),
            })
            .unwrap(),
        ),
        _ => None,
    };

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
        text_edits: None,
        data,
        tooltip,
        label,
    })
}

fn inlay_hint_label(
    snap: &GlobalStateSnapshot,
    fields_to_resolve: &InlayFieldsToResolve,
    something_to_resolve: &mut bool,
    needs_resolve: bool,
    mut label: InlayHintLabel,
) -> Cancellable<(lsp_types::InlayHintLabel, Option<lsp_types::InlayHintTooltip>)> {
    let (label, tooltip) = match &*label.parts {
        [InlayHintLabelPart { linked_location: None, .. }] => {
            let InlayHintLabelPart { text, tooltip, .. } = label.parts.pop().unwrap();
            let tooltip = tooltip.and_then(|it| match it {
                LazyProperty::Computed(it) => Some(it),
                LazyProperty::Lazy => {
                    *something_to_resolve |= needs_resolve && fields_to_resolve.resolve_hint_tooltip;
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
                            *something_to_resolve |= fields_to_resolve.resolve_label_tooltip;
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
                                *something_to_resolve |= fields_to_resolve.resolve_label_location;
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

pub(crate) fn runnable(
    snap: &GlobalStateSnapshot,
    runnable: Runnable,
) -> Cancellable<Option<lsp_ext::Runnable>> {
    let package_id = snap.analysis.package_id(runnable.nav_item.file_id)?;
    let package_manifest_file_id = match snap.analysis.manifest_file_id(package_id)? {
        Some(file_id) => file_id,
        None => {
            return Ok(None);
        }
    };
    let workspace_root = match snap
        .file_id_to_file_path(package_manifest_file_id)
        .parent()
        .and_then(|it| it.into_abs_path())
    {
        Some(path) => path,
        None => {
            return Ok(None);
        }
    };

    let config = snap.config.runnables();

    let Some(path) = snap.file_id_to_file_path(runnable.nav_item.file_id).parent() else {
        return Ok(None);
    };
    let aptos_args = match &runnable.kind {
        RunnableKind::Test { test_path } => {
            let mut args = vec!["move", "test", "--filter", test_path.as_str()]
                .iter()
                .map(|it| it.to_string())
                .collect::<Vec<_>>();
            for extra_arg in config.tests_extra_args {
                if !args.contains(&extra_arg) {
                    args.push(extra_arg);
                }
            }
            args
        }
        RunnableKind::ProveFun { only } => {
            let mut args = vec!["move", "prove", "--only", only.as_str()]
                .iter()
                .map(|it| it.to_string())
                .collect::<Vec<_>>();
            for extra_arg in config.prover_extra_args {
                if !args.contains(&extra_arg) {
                    args.push(extra_arg);
                }
            }
            args
        }
        RunnableKind::ProveModule { filter } => {
            let mut args = vec!["move", "prove", "--filter", filter.as_str()]
                .iter()
                .map(|it| it.to_string())
                .collect::<Vec<_>>();
            for extra_arg in config.prover_extra_args {
                if !args.contains(&extra_arg) {
                    args.push(extra_arg);
                }
            }
            args
        }
    };

    let label = runnable.label();
    let location = location_link(snap, None, runnable.nav_item)?;

    Ok(Some(lsp_ext::Runnable {
        label,
        location: Some(location),
        args: lsp_ext::AptosRunnableArgs {
            workspace_root: workspace_root.into(),
            args: aptos_args.iter().map(|it| it.to_string()).collect(),
            environment: Default::default(),
        },
    }))
}

pub(crate) fn code_lens(
    acc: &mut Vec<lsp_types::CodeLens>,
    snap: &GlobalStateSnapshot,
    annotation: Annotation,
) -> Cancellable<()> {
    let client_commands_config = snap.config.client_commands();
    match annotation.kind {
        AnnotationKind::Runnable(run) => {
            let line_index = snap.file_line_index(run.nav_item.file_id)?;
            let annotation_range = lsp_range(&line_index, annotation.range);

            let title = run.title();
            let r = runnable(snap, run)?;

            if let Some(r) = r {
                let lens_config = snap.config.lens();

                if lens_config.runnables && client_commands_config.run_single {
                    let command = command::run_single(&r, &title);
                    acc.push(lsp_types::CodeLens {
                        range: annotation_range,
                        command: Some(command),
                        data: None,
                    })
                }
            }
        }
        AnnotationKind::HasSpecs { pos, item_spec_refs } => {
            if !client_commands_config.show_references || !client_commands_config.goto_location {
                return Ok(());
            }
            let line_index = snap.file_line_index(pos.file_id)?;
            let annotation_range = lsp_range(&line_index, annotation.range);
            let url = url(snap, pos.file_id);
            let pos = lsp_position(&line_index, pos.offset);

            let id = lsp_types::TextDocumentIdentifier { uri: url.clone() };

            let doc_pos = lsp_types::TextDocumentPositionParams::new(id, pos);

            let goto_params = lsp_types::request::GotoImplementationParams {
                text_document_position_params: doc_pos,
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
            };

            let command = item_spec_refs
                .map(|mut nav_items| match nav_items.len() {
                    0 => None,
                    1 => {
                        let nav_item = nav_items.pop().unwrap();
                        command::goto_location(snap, "1 specification", &nav_item)
                    }
                    items_len => {
                        let locations: Vec<lsp_types::Location> = nav_items
                            .into_iter()
                            .filter_map(|target| {
                                location(
                                    snap,
                                    FileRange {
                                        file_id: target.file_id,
                                        range: target.full_range,
                                    },
                                )
                                .ok()
                            })
                            .collect();
                        Some(command::show_references(
                            format!("{items_len} specifications"),
                            &url,
                            pos,
                            locations,
                        ))
                    }
                })
                .flatten();

            acc.push(lsp_types::CodeLens {
                range: annotation_range,
                command,
                data: (|| {
                    let version = snap.url_file_version(&url)?;
                    Some(
                        serde_json::to_value(lsp_ext::CodeLensResolveData {
                            version,
                            kind: lsp_ext::CodeLensResolveDataKind::Specs(goto_params),
                        })
                        .unwrap(),
                    )
                })(),
            })
        }
    }
    Ok(())
}

pub(crate) mod command {
    use crate::global_state::GlobalStateSnapshot;
    use crate::lsp::to_proto::{location, location_link};
    use crate::lsp_ext;
    use ide::NavigationTarget;
    use syntax::files::FileRange;

    pub(crate) fn run_single(runnable: &lsp_ext::Runnable, title: &str) -> lsp_types::Command {
        lsp_types::Command {
            title: title.to_owned(),
            command: "move-on-aptos.runSingle".into(),
            arguments: Some(vec![serde_json::to_value(runnable).unwrap()]),
        }
    }

    pub(crate) fn show_references(
        title: String,
        uri: &lsp_types::Url,
        position: lsp_types::Position,
        locations: Vec<lsp_types::Location>,
    ) -> lsp_types::Command {
        // We cannot use the 'editor.action.showReferences' command directly
        // because that command requires vscode types which we convert in the handler
        // on the client side.

        lsp_types::Command {
            title,
            command: "move-on-aptos.showReferences".into(),
            arguments: Some(vec![
                serde_json::to_value(uri).unwrap(),
                serde_json::to_value(position).unwrap(),
                serde_json::to_value(locations).unwrap(),
            ]),
        }
    }

    pub(crate) fn goto_location(
        snap: &GlobalStateSnapshot,
        title: impl Into<String>,
        nav: &NavigationTarget,
    ) -> Option<lsp_types::Command> {
        let value = if snap.config.location_link() {
            let link = location_link(snap, None, nav.clone()).ok()?;
            serde_json::to_value(link).ok()?
        } else {
            let range = FileRange {
                file_id: nav.file_id,
                range: nav.focus_or_full_range(),
            };
            let location = location(snap, range).ok()?;
            serde_json::to_value(location).ok()?
        };

        Some(lsp_types::Command {
            title: title.into(),
            command: "move-on-aptos.gotoLocation".into(),
            arguments: Some(vec![value]),
        })
    }
}

pub(crate) fn rename_error(err: RenameError) -> LspError {
    // This is wrong, but we don't have a better alternative I suppose?
    // https://github.com/microsoft/language-server-protocol/issues/1341
    invalid_params_error(err.to_string())
}
