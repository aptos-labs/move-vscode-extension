use crate::Config;
use crate::global_state::GlobalStateSnapshot;
use crate::line_index::{LineEndings, LineIndex, PositionEncoding};
use crate::lsp::semantic_tokens;
use camino::{Utf8Component, Utf8Prefix};
use ide::syntax_highlighting::tags::{Highlight, HlTag};
use ide::{Cancellable, HlRange, NavigationTarget};
use ide_completion::item::{CompletionItem, CompletionItemKind};
use ide_db::text_edit::{Indel, TextEdit};
use ide_db::{Severity, SymbolKind};
use lang::files::FileRange;
use line_index::{TextRange, TextSize};
use std::ops::Not;
use std::sync::atomic::{AtomicU32, Ordering};
use stdx::itertools::Itertools;
use vfs::{AbsPath, FileId};

pub(crate) fn position(line_index: &LineIndex, offset: TextSize) -> lsp_types::Position {
    let line_col = line_index.index.line_col(offset);
    match line_index.encoding {
        PositionEncoding::Utf8 => lsp_types::Position::new(line_col.line, line_col.col),
        PositionEncoding::Wide(enc) => {
            let line_col = line_index.index.to_wide(enc, line_col).unwrap();
            lsp_types::Position::new(line_col.line, line_col.col)
        }
    }
}

pub(crate) fn range(line_index: &LineIndex, range: TextRange) -> lsp_types::Range {
    let start = position(line_index, range.start());
    let end = position(line_index, range.end());
    lsp_types::Range::new(start, end)
}

pub(crate) fn symbol_kind(symbol_kind: SymbolKind) -> lsp_types::SymbolKind {
    match symbol_kind {
        SymbolKind::Function => lsp_types::SymbolKind::FUNCTION,
        SymbolKind::Method => lsp_types::SymbolKind::METHOD,
        SymbolKind::Struct => lsp_types::SymbolKind::STRUCT,
        SymbolKind::Enum => lsp_types::SymbolKind::ENUM,
        SymbolKind::EnumVariant => lsp_types::SymbolKind::ENUM_MEMBER,
        // SymbolKind::Trait | SymbolKind::TraitAlias => lsp_types::SymbolKind::INTERFACE,
        /*SymbolKind::Macro
        | SymbolKind::ProcMacro
        | SymbolKind::BuiltinAttr*/
        SymbolKind::Attribute
        /*| SymbolKind::Derive
        | SymbolKind::DeriveHelper*/ => lsp_types::SymbolKind::FUNCTION,
        SymbolKind::Module/* | SymbolKind::ToolModule*/ => lsp_types::SymbolKind::MODULE,
        /*SymbolKind::TypeAlias |*/ SymbolKind::TypeParam /*| SymbolKind::SelfType*/ => {
            lsp_types::SymbolKind::TYPE_PARAMETER
        }
        SymbolKind::Field => lsp_types::SymbolKind::FIELD,
        // SymbolKind::Static => lsp_types::SymbolKind::CONSTANT,
        SymbolKind::Const => lsp_types::SymbolKind::CONSTANT,
        // SymbolKind::ConstParam => lsp_types::SymbolKind::CONSTANT,
        // SymbolKind::Impl => lsp_types::SymbolKind::OBJECT,
        SymbolKind::Local
        // | SymbolKind::SelfParam
        // | SymbolKind::LifetimeParam
        | SymbolKind::ValueParam
        | SymbolKind::Label => lsp_types::SymbolKind::VARIABLE,
        // SymbolKind::Union => lsp_types::SymbolKind::STRUCT,
        // SymbolKind::InlineAsmRegOrRegClass => lsp_types::SymbolKind::VARIABLE,
    }
}

pub(crate) fn diagnostic_severity(severity: Severity) -> lsp_types::DiagnosticSeverity {
    match severity {
        Severity::Error => lsp_types::DiagnosticSeverity::ERROR,
        Severity::Warning => lsp_types::DiagnosticSeverity::WARNING,
        Severity::WeakWarning => lsp_types::DiagnosticSeverity::HINT,
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
        },
    }
}

pub(crate) fn text_edit(line_index: &LineIndex, indel: Indel) -> lsp_types::TextEdit {
    let range = range(line_index, indel.delete);
    let new_text = match line_index.endings {
        LineEndings::Unix => indel.insert,
        LineEndings::Dos => indel.insert.replace('\n', "\r\n"),
    };
    lsp_types::TextEdit { range, new_text }
}

pub(crate) fn completion_text_edit(
    line_index: &LineIndex,
    insert_replace_support: Option<lsp_types::Position>,
    indel: Indel,
) -> lsp_types::CompletionTextEdit {
    let text_edit = text_edit(line_index, indel);
    match insert_replace_support {
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
        .map(|indel| self::text_edit(line_index, indel))
        .collect()
}

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
            let range = range(&line_index, src.range);
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
    let target_range = range(&line_index, target.full_range);
    let target_selection_range = target
        .focus_range
        .map(|it| range(&line_index, it))
        .unwrap_or(target_range);
    Ok((target_uri, target_range, target_selection_range))
}

pub(crate) fn location(
    snap: &GlobalStateSnapshot,
    frange: FileRange,
) -> Cancellable<lsp_types::Location> {
    let url = url(snap, frange.file_id);
    let line_index = snap.file_line_index(frange.file_id)?;
    let range = range(&line_index, frange.range);
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

        let (ty, mods) = semantic_token_type_and_modifiers(highlight_range.highlight);

        // if !non_standard_tokens {
        //     ty = match standard_fallback_type(ty) {
        //         Some(ty) => ty,
        //         None => continue,
        //     };
        //     mods.standard_fallback();
        // }
        let token_index = semantic_tokens::type_index(ty);
        let modifier_bitset = mods.0;

        for mut text_range in line_index.index.lines(highlight_range.range) {
            if text[text_range].ends_with('\n') {
                text_range = TextRange::new(text_range.start(), text_range.end() - TextSize::of('\n'));
            }
            let range = range(line_index, text_range);
            builder.push(range, token_index, modifier_bitset);
        }
    }

    builder.build()
}

fn semantic_token_type_and_modifiers(
    highlight: Highlight,
) -> (lsp_types::SemanticTokenType, semantic_tokens::ModifierSet) {
    use semantic_tokens::{/*modifiers as mods, */ types};

    let ty = match highlight.tag {
        HlTag::Symbol(symbol) => match symbol {
            SymbolKind::Attribute => types::DECORATOR,
            // SymbolKind::Derive => types::DERIVE,
            // SymbolKind::DeriveHelper => types::DERIVE_HELPER,
            SymbolKind::Module => types::NAMESPACE,
            // SymbolKind::Impl => types::TYPE_ALIAS,
            SymbolKind::Field => types::PROPERTY,
            SymbolKind::TypeParam => types::TYPE_PARAMETER,
            // SymbolKind::ConstParam => types::CONST_PARAMETER,
            // SymbolKind::LifetimeParam => types::LIFETIME,
            SymbolKind::Label => types::LABEL,
            SymbolKind::ValueParam => types::PARAMETER,
            // SymbolKind::SelfParam => types::SELF_KEYWORD,
            // SymbolKind::SelfType => types::SELF_TYPE_KEYWORD,
            SymbolKind::Local => types::VARIABLE,
            SymbolKind::Method => types::METHOD,
            SymbolKind::Function => types::FUNCTION,
            SymbolKind::Const => types::CONST,
            // SymbolKind::Static => types::STATIC,
            SymbolKind::Struct => types::STRUCT,
            SymbolKind::Enum => types::ENUM,
            SymbolKind::EnumVariant => types::ENUM_MEMBER,
            // SymbolKind::BuiltinAttr => types::BUILTIN_ATTRIBUTE,
            // SymbolKind::ToolModule => types::TOOL_MODULE,
            // SymbolKind::InlineAsmRegOrRegClass => types::KEYWORD,
        },
        // HlTag::AttributeBracket => types::ATTRIBUTE_BRACKET,
        HlTag::BoolLiteral => types::BOOLEAN,
        HlTag::BuiltinType => types::BUILTIN_TYPE,
        /*HlTag::ByteLiteral |*/
        HlTag::NumericLiteral => types::NUMBER,
        // HlTag::CharLiteral => types::CHAR,
        HlTag::Comment => types::COMMENT,
        // HlTag::EscapeSequence => types::ESCAPE_SEQUENCE,
        // HlTag::InvalidEscapeSequence => types::INVALID_ESCAPE_SEQUENCE,
        // HlTag::FormatSpecifier => types::FORMAT_SPECIFIER,
        HlTag::Keyword => types::KEYWORD,
        HlTag::None => types::GENERIC,
        // HlTag::Operator(op) => match op {
        //     HlOperator::Bitwise => types::BITWISE,
        //     HlOperator::Arithmetic => types::ARITHMETIC,
        //     HlOperator::Logical => types::LOGICAL,
        //     HlOperator::Comparison => types::COMPARISON,
        //     HlOperator::Other => types::OPERATOR,
        // },
        HlTag::StringLiteral => types::STRING,
        HlTag::UnresolvedReference => types::UNRESOLVED_REFERENCE,
        // HlTag::Punctuation(punct) => match punct {
        //     HlPunct::Bracket => types::BRACKET,
        //     HlPunct::Brace => types::BRACE,
        //     HlPunct::Parenthesis => types::PARENTHESIS,
        //     HlPunct::Angle => types::ANGLE,
        //     HlPunct::Comma => types::COMMA,
        //     HlPunct::Dot => types::DOT,
        //     HlPunct::Colon => types::COLON,
        //     HlPunct::Semi => types::SEMICOLON,
        //     HlPunct::Other => types::PUNCTUATION,
        //     HlPunct::MacroBang => types::MACRO_BANG,
        // },
    };

    let mods = semantic_tokens::ModifierSet::default();
    // for modifier in highlight.mods.iter() {
    //     let modifier = match modifier {
    //         HlMod::Associated => mods::ASSOCIATED,
    //         HlMod::Async => mods::ASYNC,
    //         HlMod::Attribute => mods::ATTRIBUTE_MODIFIER,
    //         HlMod::Callable => mods::CALLABLE,
    //         HlMod::Const => mods::CONSTANT,
    //         HlMod::Consuming => mods::CONSUMING,
    //         HlMod::ControlFlow => mods::CONTROL_FLOW,
    //         HlMod::CrateRoot => mods::CRATE_ROOT,
    //         HlMod::DefaultLibrary => mods::DEFAULT_LIBRARY,
    //         HlMod::Definition => mods::DECLARATION,
    //         HlMod::Documentation => mods::DOCUMENTATION,
    //         HlMod::Injected => mods::INJECTED,
    //         HlMod::IntraDocLink => mods::INTRA_DOC_LINK,
    //         HlMod::Library => mods::LIBRARY,
    //         HlMod::Macro => mods::MACRO_MODIFIER,
    //         HlMod::ProcMacro => mods::PROC_MACRO_MODIFIER,
    //         HlMod::Mutable => mods::MUTABLE,
    //         HlMod::Public => mods::PUBLIC,
    //         HlMod::Reference => mods::REFERENCE,
    //         HlMod::Static => mods::STATIC,
    //         HlMod::Trait => mods::TRAIT_MODIFIER,
    //         HlMod::Unsafe => mods::UNSAFE,
    //     };
    //     mods |= modifier;
    // }

    (ty, mods)
}

pub(crate) fn completion_items(
    config: &Config,
    // fields_to_resolve: &CompletionFieldsToResolve,
    line_index: &LineIndex,
    version: Option<i32>,
    tdpp: lsp_types::TextDocumentPositionParams,
    completion_trigger_character: Option<char>,
    items: Vec<CompletionItem>,
) -> Vec<lsp_types::CompletionItem> {
    // if config.completion_hide_deprecated() {
    //     items.retain(|item| !item.deprecated);
    // }

    // let max_relevance = items.iter().map(|it| it.relevance.score()).max().unwrap_or_default();
    let mut res = Vec::with_capacity(items.len());
    for item in items {
        completion_item(
            &mut res,
            config,
            // fields_to_resolve,
            line_index,
            version,
            &tdpp,
            // max_relevance,
            completion_trigger_character,
            item,
        );
    }

    // if let Some(limit) = config.completion(None).limit {
    //     res.sort_by(|item1, item2| item1.sort_text.cmp(&item2.sort_text));
    //     res.truncate(limit);
    // }

    res
}

fn completion_item(
    acc: &mut Vec<lsp_types::CompletionItem>,
    config: &Config,
    // fields_to_resolve: &CompletionFieldsToResolve,
    line_index: &LineIndex,
    version: Option<i32>,
    tdpp: &lsp_types::TextDocumentPositionParams,
    // max_relevance: u32,
    completion_trigger_character: Option<char>,
    item: CompletionItem,
) {
    let insert_replace_support = config.insert_replace_support().then_some(tdpp.position);
    // let ref_match = item.ref_match();

    let mut additional_text_edits = Vec::new();
    // let mut something_to_resolve = false;

    let filter_text = /*if fields_to_resolve.resolve_filter_text {
        something_to_resolve |= !item.lookup().is_empty();
        None
    } else*/ {
        Some(item.lookup().to_owned())
    };

    let text_edit = /*if fields_to_resolve.resolve_text_edit {
        something_to_resolve |= true;
        None
    } else*/ {
        // LSP does not allow arbitrary edits in completion, so we have to do a
        // non-trivial mapping here.
        let mut text_edit = None;
        let source_range = item.source_range;
        for indel in &item.text_edit {
            if indel.delete.contains_range(source_range) {
                // Extract this indel as the main edit
                text_edit = Some(if indel.delete == source_range {
                    completion_text_edit(line_index, insert_replace_support, indel.clone())
                } else {
                    assert_eq!(source_range.end(), indel.delete.end());
                    let range1 = TextRange::new(indel.delete.start(), source_range.start());
                    let range2 = source_range;
                    let indel1 = Indel::delete(range1);
                    let indel2 = Indel::replace(range2, indel.insert.clone());
                    additional_text_edits.push(self::text_edit(line_index, indel1));
                    completion_text_edit(line_index, insert_replace_support, indel2)
                })
            } else {
                assert!(source_range.intersect(indel.delete).is_none());
                let text_edit = self::text_edit(line_index, indel.clone());
                additional_text_edits.push(text_edit);
            }
        }
        Some(text_edit.unwrap())
    };

    let insert_text_format = item.is_snippet.then_some(lsp_types::InsertTextFormat::SNIPPET);
    // let tags = if fields_to_resolve.resolve_tags {
    //     something_to_resolve |= item.deprecated;
    //     None
    // } else {
    //     item.deprecated.then(|| vec![lsp_types::CompletionItemTag::DEPRECATED])
    // };
    // let command = if item.trigger_call_info && config.client_commands().trigger_parameter_hints {
    //     if fields_to_resolve.resolve_command {
    //         something_to_resolve |= true;
    //         None
    //     } else {
    //         Some(command::trigger_parameter_hints())
    //     }
    // } else {
    //     None
    // };

    let detail = /*if fields_to_resolve.resolve_detail {
        something_to_resolve |= item.detail.is_some();
        None
    } else*/ {
        item.detail.clone()
    };

    // let documentation = if fields_to_resolve.resolve_documentation {
    //     something_to_resolve |= item.documentation.is_some();
    //     None
    // } else {
    //     item.documentation.clone().map(documentation)
    // };

    let mut lsp_item = lsp_types::CompletionItem {
        label: item.label.primary.to_string(),
        detail,
        filter_text,
        kind: Some(completion_item_kind(item.kind)),
        text_edit,
        additional_text_edits: additional_text_edits
            .is_empty()
            .not()
            .then_some(additional_text_edits),
        // documentation,
        // deprecated: item.deprecated.then_some(item.deprecated),
        // tags,
        // command,
        insert_text_format,
        ..Default::default()
    };

    if config.completion_label_details_support() {
        let has_label_details = item.label.detail_left.is_some() || item.label.detail_right.is_some();
        /*if fields_to_resolve.resolve_label_details {
            something_to_resolve |= has_label_details;
        } else */
        if has_label_details {
            lsp_item.label_details = Some(lsp_types::CompletionItemLabelDetails {
                detail: item.label.detail_left.clone(),
                description: item.label.detail_right.clone(),
            });
        }
    } else if let Some(label_detail) = &item.label.detail_left {
        lsp_item.label.push_str(label_detail.as_str());
    }

    // set_score(&mut lsp_item, max_relevance, item.relevance);

    // let imports =
    //     if config.completion(None).enable_imports_on_the_fly && !item.import_to_add.is_empty() {
    //         item.import_to_add
    //             .clone()
    //             .into_iter()
    //             .map(|(import_path, import_name)| lsp_ext::CompletionImport {
    //                 full_import_path: import_path,
    //                 imported_name: import_name,
    //             })
    //             .collect()
    //     } else {
    //         Vec::new()
    //     };
    // let (ref_resolve_data, resolve_data) = if something_to_resolve || !imports.is_empty() {
    //     let ref_resolve_data = if ref_match.is_some() {
    //         let ref_resolve_data = lsp_ext::CompletionResolveData {
    //             position: tdpp.clone(),
    //             imports: Vec::new(),
    //             version,
    //             trigger_character: completion_trigger_character,
    //             for_ref: true,
    //             hash: BASE64_STANDARD.encode(completion_item_hash(&item, true)),
    //         };
    //         Some(to_value(ref_resolve_data).unwrap())
    //     } else {
    //         None
    //     };
    //     let resolve_data = lsp_ext::CompletionResolveData {
    //         position: tdpp.clone(),
    //         imports,
    //         version,
    //         trigger_character: completion_trigger_character,
    //         for_ref: false,
    //         hash: BASE64_STANDARD.encode(completion_item_hash(&item, false)),
    //     };
    //     (ref_resolve_data, Some(to_value(resolve_data).unwrap()))
    // } else {
    //     (None, None)
    // };

    // if let Some((label, indel, relevance)) = ref_match {
    //     let mut lsp_item_with_ref =
    //         lsp_types::CompletionItem { label, data: ref_resolve_data, ..lsp_item.clone() };
    //     lsp_item_with_ref
    //         .additional_text_edits
    //         .get_or_insert_with(Default::default)
    //         .push(self::text_edit(line_index, indel));
    //     set_score(&mut lsp_item_with_ref, max_relevance, relevance);
    //     acc.push(lsp_item_with_ref);
    // };

    // lsp_item.data = resolve_data;
    acc.push(lsp_item);

    // fn set_score(
    //     res: &mut lsp_types::CompletionItem,
    //     max_relevance: u32,
    //     relevance: CompletionRelevance,
    // ) {
    //     if relevance.is_relevant() && relevance.score() == max_relevance {
    //         res.preselect = Some(true);
    //     }
    //     // The relevance needs to be inverted to come up with a sort score
    //     // because the client will sort ascending.
    //     let sort_score = relevance.score() ^ 0xFF_FF_FF_FF;
    //     // Zero pad the string to ensure values can be properly sorted
    //     // by the client. Hex format is used because it is easier to
    //     // visually compare very large values, which the sort text
    //     // tends to be since it is the opposite of the score.
    //     res.sort_text = Some(format!("{sort_score:08x}"));
    // }
}

pub(crate) fn markup_content(markup: String) -> lsp_types::MarkupContent {
    // todo: format docs later
    lsp_types::MarkupContent {
        kind: lsp_types::MarkupKind::PlainText,
        value: markup,
    }
}
