// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::context::CompletionContext;
use crate::item::{CompletionItem, CompletionItemBuilder, CompletionItemKind, CompletionRelevance};
use ide_db::SymbolKind;
use syntax::{AstNode, SyntaxKind, ast};

pub(crate) mod function;
pub(crate) mod struct_or_enum;

pub(crate) fn render_named_item(
    ctx: &CompletionContext<'_>,
    item_name: &str,
    named_item: ast::NamedElement,
) -> CompletionItemBuilder {
    let completion_kind = item_to_kind(named_item.syntax().kind());

    let mut item = CompletionItem::new(completion_kind, ctx.source_range(), item_name);
    item.set_relevance(CompletionRelevance {
        exact_name_match: compute_exact_name_match(ctx, &item_name),
        ..CompletionRelevance::default()
    });

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
        _ => {
            tracing::info!("Unhandled completion item {:?}", kind);
            CompletionItemKind::UnresolvedReference
        }
    }
}

pub(crate) fn compute_exact_name_match(ctx: &CompletionContext<'_>, completion_name: &str) -> bool {
    ctx.expected_name
        .as_ref()
        .is_some_and(|name| name.as_string() == completion_name)
}
