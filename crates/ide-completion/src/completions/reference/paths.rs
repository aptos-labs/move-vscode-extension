// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::completions::Completions;
use crate::context::CompletionContext;
use crate::item::{CompletionItem, CompletionItemKind};
use crate::render::function::{FunctionKind, render_function};
use crate::render::new_named_item;
use crate::render::struct_or_enum::render_struct_or_enum;
use crate::render::type_owner::{render_ident_pat, render_type_owner};
use ide_db::SymbolKind;
use lang::nameres::path_kind::path_kind;
use lang::nameres::path_resolution::{ResolutionContext, get_path_resolve_variants};
use lang::nameres::scope::ScopeEntryListExt;
use lang::nameres::{labels, path_kind};
use std::cell::RefCell;
use std::collections::HashSet;
use syntax::SyntaxKind::*;
use syntax::ast::idents::PRIMITIVE_TYPES;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::ast::node_ext::syntax_element::SyntaxElementExt;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, T, algo, ast};

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn add_path_completions(
    completions: &RefCell<Completions>,
    ctx: &CompletionContext<'_>,
    original_path: Option<InFile<ast::Path>>,
    fake_path: ast::Path,
) -> Option<()> {
    let path_ctx = path_completion_ctx(ctx, &original_path, fake_path.clone())?;

    if path_ctx.qualifier.is_none() && path_ctx.path_kind == PathKind::Expr {
        add_expr_keywords(completions, ctx, &fake_path);
    }

    let acc = &mut completions.borrow_mut();

    if let Some(completion_items) = add_completions_from_the_resolution_entries(ctx, &path_ctx) {
        acc.add_all(completion_items);
    }

    if path_ctx.qualifier.is_none() {
        match path_ctx.path_kind {
            PathKind::Type => {
                if !path_ctx.is_acquires {
                    for type_name in PRIMITIVE_TYPES.iter() {
                        if *type_name == "vector" {
                            let mut item = ctx.new_item(CompletionItemKind::BuiltinType, "vector");
                            item.insert_snippet("vector<$0>");
                            acc.add(item.build(ctx.db));
                            continue;
                        }
                        acc.add(ctx.new_snippet_item(
                            CompletionItemKind::BuiltinType,
                            format!("{type_name}$0"),
                        ));
                    }
                }
            }
            PathKind::Expr => {
                // vector literal
                acc.add(ctx.new_snippet_item(CompletionItemKind::Keyword, "vector[$0]"));

                // assert!
                let mut item = ctx.new_item(
                    CompletionItemKind::SymbolKind(SymbolKind::Assert),
                    "assert!(_: bool, err: u64)",
                );
                item.insert_snippet("assert!($0)");
                acc.add(item.build(ctx.db));
            }
            _ => (),
        }
    }

    Some(())
}

fn add_completions_from_the_resolution_entries(
    ctx: &CompletionContext<'_>,
    path_ctx: &PathCompletionCtx,
) -> Option<Vec<CompletionItem>> {
    let original_file = ctx.original_file()?;

    let path_kind = match path_ctx.original_path.clone() {
        Some(original_path) => {
            path_kind(original_path.value.qualifier(), original_path.value.clone(), true)?
        }
        None => {
            let original_qualifier = path_ctx.original_qualifier(&original_file);
            let fake_path_kind = path_kind(original_qualifier, path_ctx.fake_path.clone(), true)?;
            if matches!(fake_path_kind, path_kind::PathKind::FieldShorthand { .. }) {
                return None;
            }
            fake_path_kind
        }
    };
    tracing::debug!(?path_kind);

    let original_start_at = path_ctx
        .original_path
        .as_ref()
        .map(|it| it.syntax())
        .or_else(|| {
            let original_start_at_from_fake = algo::ancestors_at_offset(
                original_file.syntax(),
                path_ctx.fake_path.syntax().text_range().start(),
            )
            .next()?
            .in_file(ctx.position.file_id);
            Some(original_start_at_from_fake)
        })?;

    let resolution_ctx = ResolutionContext {
        start_at: original_start_at.clone(),
        is_completion: true,
    };
    let entries = get_path_resolve_variants(ctx.db, &resolution_ctx, path_kind.clone());

    let mut visible_entries = entries.filter_by_visibility(ctx.db, &original_start_at.clone());
    tracing::debug!(completion_item_entries = ?visible_entries);

    // remove already present items in use group
    if let Some(use_group) = path_ctx.original_use_group(&original_file) {
        let speck_names = use_group
            .use_specks()
            .filter(|it| !it.syntax().text_range().contains_inclusive(ctx.original_offset()))
            .filter_map(|it| it.path_name())
            .collect::<HashSet<_>>();
        visible_entries.retain(|it| !speck_names.contains(&it.name));
    }

    let mut completion_items = vec![];
    for entry in visible_entries {
        let name = entry.name.clone();
        let named_item = entry.cast_into::<ast::NamedElement>(ctx.db)?;
        let named_item_kind = named_item.kind();

        // in acquires, only structs and enums are allowed
        if path_ctx.is_acquires && !matches!(named_item_kind, STRUCT | ENUM) {
            continue;
        }

        match named_item_kind {
            FUN | SPEC_FUN | SPEC_INLINE_FUN => {
                completion_items.push(
                    render_function(
                        ctx,
                        path_ctx.is_use_stmt(),
                        path_ctx.has_any_parens(),
                        name,
                        named_item.cast_into::<ast::AnyFun>()?,
                        FunctionKind::Fun,
                        None,
                    )
                    .build(ctx.db),
                );
            }
            STRUCT | ENUM => {
                completion_items.push(
                    render_struct_or_enum(ctx, name, named_item.cast_into::<ast::StructOrEnum>()?)
                        .build(ctx.db),
                );
            }
            IDENT_PAT => {
                let ident_pat = named_item.cast_into::<ast::IdentPat>()?;
                completion_items.push(render_ident_pat(ctx, &name, ident_pat).build(ctx.db));
            }
            GLOBAL_VARIABLE_DECL => {
                let global_var = named_item.cast_into::<ast::GlobalVariableDecl>()?;
                let item = render_type_owner(ctx, &name, global_var.map_into());
                completion_items.push(item.build(ctx.db));
            }
            _ => {
                let item = new_named_item(ctx, &name, named_item_kind);
                completion_items.push(item.build(ctx.db));
            }
        }
    }
    Some(completion_items)
}

pub(crate) fn add_expr_keywords(
    completions: &RefCell<Completions>,
    ctx: &CompletionContext<'_>,
    fake_path: &ast::Path,
) -> Option<()> {
    let mut acc = completions.borrow_mut();

    acc.add(ctx.new_snippet_keyword("if $0"));
    acc.add(ctx.new_snippet_keyword("match $0"));
    acc.add(ctx.new_snippet_keyword("loop $0"));
    acc.add(ctx.new_snippet_keyword("while $0"));
    acc.add(ctx.new_snippet_keyword("for $0"));
    acc.add(ctx.new_snippet_keyword("let $0"));

    acc.add(ctx.new_snippet_keyword("true$0"));
    acc.add(ctx.new_snippet_keyword("false$0"));

    if !labels::loop_ancestors(&ctx.original_token.clone().into()).is_empty() {
        acc.add(ctx.new_snippet_keyword("continue$0"));
        acc.add(ctx.new_snippet_keyword("break$0"));
    }

    let is_msl_expr = fake_path.syntax().is_msl_context();
    if !is_msl_expr {
        return Some(());
    }

    // only direct path of ExprStmt / BlockExpr
    let parent = fake_path.path_expr()?.syntax().parent()?.kind();
    if matches!(parent, EXPR_STMT | BLOCK_EXPR) {
        acc.add(ctx.new_snippet_keyword("assert $0"));
        acc.add(ctx.new_snippet_keyword("assume $0"));
        acc.add(ctx.new_snippet_keyword("requires $0"));
        acc.add(ctx.new_snippet_keyword("decreases $0"));
        acc.add(ctx.new_snippet_keyword("ensures $0"));
        acc.add(ctx.new_snippet_keyword("modifies $0"));
        acc.add(ctx.new_snippet_keyword("include $0"));
        acc.add(ctx.new_snippet_keyword("apply $0"));
        acc.add(ctx.new_snippet_keyword("aborts_if $0"));
        acc.add(ctx.new_snippet_keyword("aborts_with $0"));
        acc.add(ctx.new_snippet_keyword("emits $0"));
        acc.add(ctx.new_snippet_keyword("axiom $0"));
        acc.add(ctx.new_snippet_keyword("pragma $0"));
    }

    Some(())
}

/// The state of the path we are currently completing.
#[derive(Debug)]
pub(crate) struct PathCompletionCtx {
    /// If this is a call with () already there (or {} in case of record patterns)
    pub(crate) has_call_parens: bool,
    /// Whether the path segment has type args or not.
    pub(crate) has_type_args: bool,
    pub(crate) is_acquires: bool,
    /// The qualifier of the current path.
    pub(crate) qualifier: Option<InFile<ast::Path>>,
    // /// The parent of the path we are completing.
    // pub(crate) parent: Option<ast::Path>,
    pub(crate) fake_path: ast::Path,
    /// The path of which we are completing the segment
    pub(crate) original_path: Option<InFile<ast::Path>>,
    pub(crate) path_kind: PathKind,
}

impl PathCompletionCtx {
    pub fn original_qualifier(&self, original_file: &ast::SourceFile) -> Option<ast::Path> {
        let fake_qualifier = self.fake_path.qualifier()?;
        original_file.find_original_node(fake_qualifier)
    }

    pub fn has_any_parens(&self) -> bool {
        self.has_call_parens || self.has_type_args
    }

    pub fn original_use_group(&self, original_file: &ast::SourceFile) -> Option<ast::UseGroup> {
        if !self.is_use_stmt() {
            return None;
        }
        let fake_use_speck = self.fake_path.root_parent_of_type::<ast::UseSpeck>()?;
        let fake_use_group = fake_use_speck.syntax().parent_of_type::<ast::UseGroup>()?;
        original_file.find_original_node(fake_use_group)
    }

    pub fn is_use_stmt(&self) -> bool {
        self.path_kind == PathKind::Use
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum PathKind {
    Expr,
    Type,
    Use,
}

fn path_completion_ctx(
    ctx: &CompletionContext<'_>,
    original_path: &Option<InFile<ast::Path>>,
    fake_path: ast::Path,
) -> Option<PathCompletionCtx> {
    let original_ident_token = original_path
        .as_ref()
        .and_then(|it| it.value.segment())
        .and_then(|it| it.name_ref())
        .and_then(|it| it.ident_token());

    let has_type_args = original_ident_token
        .clone()
        .and_then(|it| it.next_token_no_trivia())
        .is_some_and(|it| it.kind() == T![<]);
    let has_call_parens = original_ident_token
        .clone()
        .and_then(|it| it.next_token_no_trivia())
        .is_some_and(|it| it.kind() == T!['(']);

    let fake_path_parent = fake_path.root_path().syntax().parent()?;
    let path_kind = match fake_path_parent.kind() {
        USE_SPECK => PathKind::Use,
        PATH_TYPE => PathKind::Type,
        PATH_EXPR => PathKind::Expr,
        _ => {
            return None;
        }
    };
    let mut qualifier: Option<InFile<ast::Path>> = None;
    if let Some(fake_path_qualifier) = fake_path.qualifier() {
        let original_file = ctx.original_file()?;
        let original_qualifier = algo::find_node_at_offset::<ast::Path>(
            original_file.syntax(),
            fake_path_qualifier.syntax().text_range().start(),
        );
        qualifier = original_qualifier.map(|it| it.in_file(ctx.position.file_id));
    }
    let is_acquires = fake_path_parent
        .cast::<ast::PathType>()
        .is_some_and(|it| it.syntax().parent_is::<ast::Acquires>());
    Some(PathCompletionCtx {
        has_call_parens,
        has_type_args,
        is_acquires,
        path_kind,
        qualifier,
        fake_path,
        original_path: original_path.clone(),
    })
}
