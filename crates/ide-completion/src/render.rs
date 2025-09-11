// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::context::CompletionContext;
use crate::item::{
    CompletionItem, CompletionItemBuilder, CompletionItemKind, CompletionRelevance,
    CompletionRelevanceTypeMatch,
};
use ide_db::SymbolKind;
use lang::types::ty::Ty;
use syntax::SyntaxKind;

pub(crate) mod function;
pub(crate) mod struct_or_enum;
pub(crate) mod type_owner;

pub(crate) fn new_named_item(
    ctx: &CompletionContext<'_>,
    item_name: &str,
    item_kind: SyntaxKind,
) -> CompletionItemBuilder {
    let _p = tracing::debug_span!("render::new_named_item").entered();

    let completion_item_kind = item_to_kind(item_kind);

    let mut item = CompletionItem::new(completion_item_kind, ctx.source_range(), item_name);
    item.set_relevance(CompletionRelevance {
        exact_name_match: compute_exact_name_match(ctx, &item_name),
        ..CompletionRelevance::default()
    });
    item.insert_snippet(format!("{item_name}$0"));

    item
}

pub(crate) fn item_to_kind(kind: SyntaxKind) -> CompletionItemKind {
    use syntax::SyntaxKind::*;
    match kind {
        MODULE => CompletionItemKind::SymbolKind(SymbolKind::Module),
        ATTR => CompletionItemKind::SymbolKind(SymbolKind::Attribute),
        FUN => CompletionItemKind::SymbolKind(SymbolKind::Function),
        SPEC_FUN => CompletionItemKind::SymbolKind(SymbolKind::Function),
        SPEC_INLINE_FUN => CompletionItemKind::SymbolKind(SymbolKind::Function),
        CONST => CompletionItemKind::SymbolKind(SymbolKind::Const),
        STRUCT => CompletionItemKind::SymbolKind(SymbolKind::Struct),
        ENUM => CompletionItemKind::SymbolKind(SymbolKind::Enum),
        IDENT_PAT => CompletionItemKind::SymbolKind(SymbolKind::Local),
        LABEL => CompletionItemKind::SymbolKind(SymbolKind::Label),
        TYPE_PARAM => CompletionItemKind::SymbolKind(SymbolKind::TypeParam),
        NAMED_FIELD => CompletionItemKind::SymbolKind(SymbolKind::Field),
        VARIANT => CompletionItemKind::SymbolKind(SymbolKind::EnumVariant),
        SCHEMA => CompletionItemKind::SymbolKind(SymbolKind::Schema),
        _ => {
            tracing::info!("Unhandled completion item {:?}", kind);
            CompletionItemKind::UnresolvedReference
        }
    }
}

pub(crate) fn compute_type_match(
    ctx: &CompletionContext<'_>,
    item_ty: Ty,
) -> Option<CompletionRelevanceTypeMatch> {
    let _p = tracing::debug_span!("render::compute_type_match").entered();

    let expected_ty = ctx.expected_type.as_ref()?;

    // We don't ever consider unit type to be an exact type match, since
    // nearly always this is not meaningful to the user.
    if matches!(expected_ty, Ty::Unit) {
        return None;
    }

    if expected_ty == &item_ty {
        Some(CompletionRelevanceTypeMatch::Exact)
    } else if ctx.sema.is_tys_compatible(item_ty, expected_ty.clone(), true) {
        Some(CompletionRelevanceTypeMatch::CouldUnify)
    } else {
        None
    }
}

pub(crate) fn compute_exact_name_match(ctx: &CompletionContext<'_>, completion_name: &str) -> bool {
    ctx.expected_name
        .as_ref()
        .is_some_and(|name| name.as_string() == completion_name)
}
