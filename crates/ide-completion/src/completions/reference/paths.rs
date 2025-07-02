use crate::completions::Completions;
use crate::context::CompletionContext;
use crate::item::{CompletionItem, CompletionItemKind};
use crate::render::function::{FunctionKind, render_function};
use crate::render::render_named_item;
use lang::nameres::labels;
use lang::nameres::path_kind::path_kind;
use lang::nameres::path_resolution::{ResolutionContext, get_path_resolve_variants};
use lang::nameres::scope::ScopeEntryListExt;
use std::cell::RefCell;
use syntax::SyntaxKind::*;
use syntax::ast::idents::PRIMITIVE_TYPES;
use syntax::ast::node_ext::syntax_element::SyntaxElementExt;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, SyntaxKind, T, ast};

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn add_path_completions(
    completions: &RefCell<Completions>,
    ctx: &CompletionContext<'_>,
    context_path: InFile<ast::Path>,
) -> Option<()> {
    let acc = &mut completions.borrow_mut();

    let path_ctx = path_completion_ctx(&context_path);

    if let Some(completion_items) = add_completions_from_the_resolution_entries(ctx, &path_ctx) {
        acc.add_all(completion_items);
    }

    match path_ctx.kind {
        PathKind::Type => {
            for primitive_type in PRIMITIVE_TYPES.iter() {
                let mut completion_item = CompletionItem::new(
                    CompletionItemKind::BuiltinType,
                    ctx.source_range(),
                    primitive_type.clone(),
                );
                completion_item.insert_snippet(format!("{}$0", primitive_type));
                acc.add(completion_item.build(ctx.db));
            }
        }
        _ => (),
    }

    Some(())
}

fn add_completions_from_the_resolution_entries(
    ctx: &CompletionContext<'_>,
    path_ctx: &PathCompletionCtx,
) -> Option<Vec<CompletionItem>> {
    let context_path = path_ctx.context_path.clone()?;

    let path_kind = path_kind(context_path.value.clone(), true)?;
    tracing::debug!(?path_kind);

    let resolution_ctx = ResolutionContext {
        path: context_path.clone(),
        is_completion: true,
    };
    let entries = get_path_resolve_variants(ctx.db, &resolution_ctx, path_kind.clone())
        .filter_by_visibility(ctx.db, &context_path.clone().map_into());
    tracing::debug!(completion_item_entries = ?entries);

    let mut completion_items = vec![];
    for entry in entries {
        let name = entry.name.clone();
        let named_item = entry.cast_into::<ast::NamedElement>(ctx.db)?;
        match named_item.kind() {
            FUN | SPEC_FUN | SPEC_INLINE_FUN => {
                completion_items.push(
                    render_function(
                        ctx,
                        path_ctx,
                        name,
                        named_item.cast_into::<ast::AnyFun>()?,
                        FunctionKind::Fun,
                        None,
                    )
                    .build(ctx.db),
                );
                continue;
            }
            _ => {
                let mut item = render_named_item(ctx, &name, named_item);
                item.insert_snippet(format!("{name}$0"));
                completion_items.push(item.build(ctx.db));
            }
        }
    }
    Some(completion_items)
}

pub(crate) fn add_expr_keywords(completions: &RefCell<Completions>, ctx: &CompletionContext<'_>) {
    let add_keyword = |kw, right_shift| {
        let mut comps = completions.borrow_mut();
        let shift = if right_shift { " " } else { "" };
        comps.add_keyword_snippet(ctx, kw, &format!("{kw}{shift}$0"));
    };

    add_keyword("if", true);
    add_keyword("match", true);
    add_keyword("loop", true);
    add_keyword("while", true);
    add_keyword("for", true);
    add_keyword("let", true);

    add_keyword("true", false);
    add_keyword("false", false);

    if !labels::loop_ancestors(&ctx.original_token.clone().into()).is_empty() {
        add_keyword("continue", false);
        add_keyword("break", false);
    }
}

/// The state of the path we are currently completing.
#[derive(Debug)]
pub(crate) struct PathCompletionCtx {
    /// If this is a call with () already there (or {} in case of record patterns)
    pub(crate) has_call_parens: bool,
    /// Whether the path segment has type args or not.
    pub(crate) has_type_args: bool,
    // /// The qualifier of the current path.
    // pub(crate) qualified: Qualified,
    // /// The parent of the path we are completing.
    // pub(crate) parent: Option<ast::Path>,
    // /// The path of which we are completing the segment
    // pub(crate) path: ast::Path,
    /// The path of which we are completing the segment in the original file
    pub(crate) context_path: Option<InFile<ast::Path>>,
    pub(crate) kind: PathKind,
}

impl Default for PathCompletionCtx {
    fn default() -> Self {
        PathCompletionCtx {
            has_call_parens: false,
            has_type_args: false,
            context_path: None,
            kind: PathKind::Expr,
        }
    }
}

impl PathCompletionCtx {
    pub fn has_any_parens(&self) -> bool {
        self.has_call_parens || self.has_type_args
    }
    pub fn is_use_stmt(&self) -> bool {
        self.kind == PathKind::Use
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum PathKind {
    Expr,
    Type,
    Use,
}

fn path_completion_ctx(path: &InFile<ast::Path>) -> PathCompletionCtx {
    let (file_id, path) = path.unpack_ref();

    let ident_token = path
        .segment()
        .and_then(|it| it.name_ref())
        .and_then(|it| it.ident_token());

    let has_type_args = ident_token
        .clone()
        .and_then(|it| it.next_token_no_trivia())
        .is_some_and(|it| it.kind() == T![<]);
    let has_call_parens = ident_token
        .clone()
        .and_then(|it| it.next_token_no_trivia())
        .is_some_and(|it| it.kind() == T!['(']);

    let path_parent = path.syntax().parent().map(|it| it.kind());
    let path_kind = match path_parent {
        Some(USE_SPECK) => PathKind::Use,
        Some(PATH_TYPE) => PathKind::Type,
        _ => PathKind::Expr,
    };
    PathCompletionCtx {
        has_call_parens,
        has_type_args,
        kind: path_kind,
        context_path: Some(path.clone().in_file(file_id)),
    }
}
