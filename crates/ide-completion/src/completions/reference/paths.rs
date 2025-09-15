// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::completions::Completions;
use crate::context::CompletionContext;
use crate::item::{CompletionItem, CompletionItemBuilder, CompletionItemKind, CompletionRelevance};
use crate::render::function::{FunctionKind, render_function};
use crate::render::new_named_item;
use crate::render::struct_or_enum::{render_schema, render_struct_or_enum};
use crate::render::type_owner::{render_ident_pat, render_type_owner};
use ide_db::SymbolKind;
use ide_db::defs::BUILTIN_MUT_RESOURCE_FUNCTIONS;
use lang::hir_db;
use lang::nameres::fq_named_element::ItemFQNameOwner;
use lang::nameres::is_visible::is_visible_in_context;
use lang::nameres::name_resolution::WalkScopesCtx;
use lang::nameres::namespaces::NONE;
use lang::nameres::path_kind::path_kind;
use lang::nameres::path_resolution::{ResolutionContext, get_path_resolve_variants};
use lang::nameres::scope::ScopeEntry;
use lang::nameres::{labels, path_kind};
use std::cell::RefCell;
use std::collections::HashSet;
use syntax::SyntaxKind::*;
use syntax::ast::HasAttrs;
use syntax::ast::idents::PRIMITIVE_TYPES;
use syntax::ast::node_ext::syntax_element::SyntaxElementExt;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, SyntaxNode, T, algo, ast};

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn add_path_completions(
    completions: &RefCell<Completions>,
    ctx: &CompletionContext<'_>,
    fake_path: ast::Path,
) -> Option<()> {
    let original_path = ctx
        .original_file()?
        .find_node_at_offset::<ast::Path>(ctx.position.offset)
        .map(|it| it.in_file(ctx.position.file_id));
    let path_ctx = path_completion_ctx(ctx, &original_path, fake_path.clone())?;

    if !path_ctx.has_qualifier && path_ctx.path_kind == PathKind::Expr {
        add_expr_keywords(completions, ctx, &path_ctx);
    }

    let acc = &mut completions.borrow_mut();

    if let Some(completion_items) = add_path_completions_from_the_resolution_entries(ctx, &path_ctx) {
        acc.add_all(completion_items);
    }

    if !path_ctx.has_qualifier {
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
                if !path_ctx.is_msl() {
                    // assert!
                    let mut assert_item = ctx.new_item(
                        CompletionItemKind::SymbolKind(SymbolKind::Assert),
                        "assert!(_: bool, err: u64)",
                    );
                    assert_item.insert_snippet("assert!($0)");
                    assert_item.lookup_by("assert");
                    acc.add(assert_item.build(ctx.db));
                }
            }
            _ => (),
        }
    }

    Some(())
}

fn add_path_completions_from_the_resolution_entries(
    ctx: &CompletionContext<'_>,
    path_ctx: &PathCompletionCtx,
) -> Option<Vec<CompletionItem>> {
    let _p = tracing::debug_span!("add_path_completions_from_the_resolution_entries").entered();

    let file_id = ctx.position.file_id;
    let original_file = ctx.original_file()?;

    let path_kind = match path_ctx.original_path.clone() {
        Some(original_path) => path_kind(
            ctx.db,
            original_path.value.qualifier(),
            &original_path.value,
            true,
        )?,
        None => {
            let original_qualifier = path_ctx.original_qualifier(&original_file);
            let fake_path_kind = path_kind(ctx.db, original_qualifier, &path_ctx.fake_path, true)?;
            if matches!(fake_path_kind, path_kind::PathKind::FieldShorthand { .. }) {
                return None;
            }
            fake_path_kind
        }
    };
    tracing::debug!(?path_kind);

    let original_start_at = path_ctx.original_start_at(&original_file)?.in_file(file_id);
    let resolution_ctx = ResolutionContext {
        start_at: original_start_at.clone(),
        is_completion: true,
    };
    let walk_ctx = WalkScopesCtx {
        allowed_ns: NONE,
        start_at: original_start_at.clone(),
        expected_name: None,
    };
    let entries = get_path_resolve_variants(ctx.db, &resolution_ctx, path_kind.clone(), walk_ctx);

    let mut visible_entries = entries
        .into_iter()
        .filter(|it| is_visible_in_context(ctx.db, it, original_start_at.clone()))
        .collect::<Vec<_>>();
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

    // remove resource builtin functions in specs
    if path_ctx.is_msl() {
        visible_entries.retain(|it| !BUILTIN_MUT_RESOURCE_FUNCTIONS.contains(it.name.as_str()));
    }

    let mut completion_items = vec![];
    for entry in visible_entries.clone() {
        if let Some(completion_item) = render_scope_entry(ctx, path_ctx, &entry) {
            completion_items.push(completion_item.build(ctx.db));
        }
    }

    if ctx.config.enable_imports_on_the_fly {
        let out_of_scope_items = add_out_of_scope_completion_items(
            ctx,
            path_ctx,
            path_kind,
            original_start_at,
            &visible_entries,
        )
        .unwrap_or_default();
        completion_items.extend(out_of_scope_items);
    }

    Some(completion_items)
}

fn add_out_of_scope_completion_items(
    ctx: &CompletionContext<'_>,
    path_ctx: &PathCompletionCtx,
    path_kind: path_kind::PathKind,
    original_start_at: InFile<SyntaxNode>,
    existing_entries: &Vec<ScopeEntry>,
) -> Option<Vec<CompletionItem>> {
    let _p = tracing::debug_span!("add_out_of_scope_completion_items").entered();

    if path_ctx.is_use_stmt() || path_ctx.is_acquires {
        return None;
    }
    let unqualified_nsset = path_kind.unqualified_ns()?;
    let import_candidates = hir_db::import_candidates(ctx.db, original_start_at.file_id)
        .iter()
        .filter(|it| unqualified_nsset.contains(it.ns))
        .filter(|it| is_visible_in_context(ctx.db, it, original_start_at.clone()));
    let mut completion_items = vec![];
    for import_candidate in import_candidates {
        if !existing_entries.contains(import_candidate) {
            if let Some(mut completion_item) = render_scope_entry(ctx, path_ctx, import_candidate) {
                if let Some(fq_name) = import_candidate
                    .cast_into::<ast::NamedElement>(ctx.db)
                    .and_then(|it| it.fq_name(ctx.db))
                {
                    completion_item.add_import(fq_name.fq_identifier_text());
                    completion_item
                        .with_relevance(|r| CompletionRelevance { is_out_of_scope: true, ..r });
                    completion_items.push(completion_item.build(ctx.db));
                }
            }
        }
    }
    Some(completion_items)
}

fn render_scope_entry(
    ctx: &CompletionContext<'_>,
    path_ctx: &PathCompletionCtx,
    scope_entry: &ScopeEntry,
) -> Option<CompletionItemBuilder> {
    let _p = tracing::debug_span!("render_scope_entry").entered();

    let name = scope_entry.name.clone();
    let named_item = scope_entry.cast_into::<ast::NamedElement>(ctx.db)?;
    let named_item_kind = named_item.kind();

    // in acquires, only structs and enums are allowed
    if path_ctx.is_acquires && !matches!(named_item_kind, STRUCT | ENUM) {
        return None;
    }

    let comp_item = match named_item_kind {
        FUN | SPEC_FUN | SPEC_INLINE_FUN => {
            let fun = named_item.cast_into::<ast::AnyFun>()?;
            if fun.value.has_attr_item("test") {
                return None;
            }
            render_function(
                ctx,
                path_ctx.is_use_stmt(),
                path_ctx.has_any_parens(),
                &name,
                fun,
                FunctionKind::Fun,
                None,
            )
        }
        STRUCT | ENUM => {
            render_struct_or_enum(ctx, name, path_ctx, named_item.cast_into::<ast::StructOrEnum>()?)
        }
        SCHEMA => render_schema(ctx, name, named_item.cast_into::<ast::Schema>()?),
        IDENT_PAT => {
            let ident_pat = named_item.cast_into::<ast::IdentPat>()?;
            render_ident_pat(ctx, &name, ident_pat)
        }
        GLOBAL_VARIABLE_DECL => {
            let global_var = named_item.cast_into::<ast::GlobalVariableDecl>()?;
            render_type_owner(ctx, &name, global_var.map_into())
        }
        _ => new_named_item(ctx, &name, named_item_kind),
    };
    Some(comp_item)
}

pub(crate) fn add_expr_keywords(
    completions: &RefCell<Completions>,
    ctx: &CompletionContext<'_>,
    path_ctx: &PathCompletionCtx,
) -> Option<()> {
    let mut acc = completions.borrow_mut();

    acc.add(ctx.new_snippet_keyword("if $0"));
    acc.add(ctx.new_snippet_keyword("match $0"));
    acc.add(ctx.new_snippet_keyword("loop $0"));
    acc.add(ctx.new_snippet_keyword("while $0"));
    acc.add(ctx.new_snippet_keyword("for $0"));
    if path_ctx.is_stmt_start {
        acc.add(ctx.new_snippet_keyword("let $0"));
    }
    acc.add(ctx.new_snippet_keyword("true$0"));
    acc.add(ctx.new_snippet_keyword("false$0"));

    if !labels::loop_ancestors(&ctx.original_token.clone().into()).is_empty() {
        acc.add(ctx.new_snippet_keyword("continue$0"));
        acc.add(ctx.new_snippet_keyword("break$0"));
    }

    if path_ctx.is_stmt_start {
        match path_ctx.msl_context {
            MslContext::CodeSpec => {
                acc.add(ctx.new_snippet_keyword("assume $0"));
                acc.add(ctx.new_snippet_keyword("assert $0"));
                acc.add(ctx.new_snippet_keyword("invariant $0"));
            }
            MslContext::ItemSpec => {
                acc.add(ctx.new_snippet_keyword("pragma $0"));
                acc.add(ctx.new_snippet_keyword("requires $0"));
                acc.add(ctx.new_snippet_keyword("decreases $0"));
                acc.add(ctx.new_snippet_keyword("ensures $0"));
                acc.add(ctx.new_snippet_keyword("modifies $0"));
                acc.add(ctx.new_snippet_keyword("include $0"));
                acc.add(ctx.new_snippet_keyword("apply $0"));
                acc.add(ctx.new_snippet_keyword("aborts_if $0"));
                acc.add(ctx.new_snippet_keyword("aborts_with $0"));
                acc.add(ctx.new_snippet_keyword("emits $0"));
                acc.add(ctx.new_snippet_keyword("invariant $0"));
            }
            MslContext::ModuleItemSpec => {
                acc.add(ctx.new_snippet_keyword("pragma $0"));
                acc.add(ctx.new_snippet_keyword("axiom $0"));
                acc.add(ctx.new_snippet_keyword("invariant $0"));
            }
            _ => (),
        }
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
    pub(crate) has_qualifier: bool,
    // pub(crate) qualifier: Option<InFile<ast::Path>>,
    // /// The parent of the path we are completing.
    // pub(crate) parent: Option<ast::Path>,
    pub(crate) fake_path: ast::Path,
    /// The path of which we are completing the segment
    pub(crate) original_path: Option<InFile<ast::Path>>,
    pub(crate) path_kind: PathKind,
    pub(crate) is_stmt_start: bool,
    pub(crate) msl_context: MslContext,
}

impl PathCompletionCtx {
    pub fn original_qualifier(&self, original_file: &ast::SourceFile) -> Option<ast::Path> {
        let fake_qualifier = self.fake_path.qualifier()?;
        original_file.find_original_node(fake_qualifier)
    }

    pub fn original_start_at(&self, original_file: &ast::SourceFile) -> Option<SyntaxNode> {
        self.original_path
            .as_ref()
            .map(|it| it.value.syntax().clone())
            .or_else(|| {
                let original_start_at_from_fake = algo::ancestors_at_offset(
                    original_file.syntax(),
                    self.fake_path.syntax().text_range().start(),
                )
                .next();
                original_start_at_from_fake
            })
    }

    pub fn original_use_group(&self, original_file: &ast::SourceFile) -> Option<ast::UseGroup> {
        if !self.is_use_stmt() {
            return None;
        }
        let fake_use_speck = self.fake_path.root_parent_of_type::<ast::UseSpeck>()?;
        let fake_use_group = fake_use_speck.syntax().parent_of_type::<ast::UseGroup>()?;
        original_file.find_original_node(fake_use_group)
    }

    pub fn has_any_parens(&self) -> bool {
        self.has_call_parens || self.has_type_args
    }

    pub fn is_use_stmt(&self) -> bool {
        self.path_kind == PathKind::Use
    }

    pub fn is_msl(&self) -> bool {
        self.msl_context != MslContext::None
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum MslContext {
    None,
    CodeSpec,
    ItemSpec,
    ModuleItemSpec,
    SpecFun,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum PathKind {
    Expr,
    Type,
    Use,
    SchemaLit,
}

fn path_completion_ctx(
    _ctx: &CompletionContext<'_>,
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
        SCHEMA_LIT => PathKind::SchemaLit,
        _ => {
            return None;
        }
    };
    let is_acquires = fake_path_parent
        .cast::<ast::PathType>()
        .is_some_and(|it| it.syntax().parent_is::<ast::Acquires>());

    let pkind = fake_path
        .syntax()
        .parent_of_type::<ast::PathExpr>()
        .and_then(|it| it.syntax().parent().map(|p| p.kind()));
    let is_stmt_start = matches!(pkind, Some(EXPR_STMT | BLOCK_EXPR));

    let mut msl_context = MslContext::None;
    if fake_path.syntax().is_msl_context() {
        if let Some(item_spec) = fake_path.syntax().ancestor_strict::<ast::ItemSpec>() {
            if item_spec.item_spec_ref().is_some() {
                msl_context = MslContext::ItemSpec;
            } else {
                msl_context = MslContext::ModuleItemSpec;
            }
        } else {
            if fake_path.syntax().has_ancestor_strict::<ast::SpecBlockExpr>() {
                msl_context = MslContext::CodeSpec;
            } else {
                msl_context = MslContext::SpecFun;
            }
        }
    }

    Some(PathCompletionCtx {
        has_call_parens,
        has_type_args,
        is_acquires,
        path_kind,
        has_qualifier: fake_path.qualifier().is_some(),
        fake_path,
        original_path: original_path.clone(),
        is_stmt_start,
        msl_context,
    })
}
