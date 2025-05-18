use crate::completions::Completions;
use crate::context::CompletionContext;
use crate::render::function::{FunctionKind, render_function};
use crate::render::render_named_item;
use lang::nameres::path_kind::{PathKind, path_kind};
use lang::nameres::path_resolution::{ResolutionContext, get_path_resolve_variants};
use lang::nameres::scope::ScopeEntryListExt;
use std::cell::RefCell;
use syntax::SyntaxKind::*;
use syntax::ast::node_ext::syntax_element::SyntaxElementExt;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::InFile;
use syntax::{AstNode, T, ast};

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn add_path_completions(
    completions: &RefCell<Completions>,
    ctx: &CompletionContext<'_>,
    context_path: InFile<ast::Path>,
) -> Option<()> {
    let path_kind = path_kind(context_path.clone().value, true)?;
    tracing::debug!(?path_kind);

    if matches!(path_kind, PathKind::Unqualified { .. }) {
        add_keywords(completions, ctx);
    }

    let acc = &mut completions.borrow_mut();

    let resolution_ctx = ResolutionContext {
        path: context_path.clone(),
        is_completion: true,
    };
    let entries = get_path_resolve_variants(ctx.db, &resolution_ctx, path_kind.clone())
        .filter_by_visibility(ctx.db, &context_path);
    tracing::debug!(completion_item_entries = ?entries);

    let path_ctx = path_completion_ctx(&context_path);

    for entry in entries {
        let name = entry.name.clone();
        let named_item = entry.cast_into::<ast::AnyNamedElement>(ctx.db)?;
        match named_item.kind() {
            FUN | SPEC_FUN | SPEC_INLINE_FUN => {
                acc.add(
                    render_function(
                        ctx,
                        &path_ctx,
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
                acc.add(render_named_item(ctx, name, named_item).build(ctx.db));
            }
        }
    }

    Some(())
}

fn add_keywords(completions: &RefCell<Completions>, ctx: &CompletionContext<'_>) {
    let add_keyword = |kw| completions.borrow_mut().add_keyword(ctx, kw);
    let add_keyword_with_shift = |kw| {
        completions
            .borrow_mut()
            .add_keyword_snippet(ctx, kw, &format!("{} $0", kw))
    };
    add_keyword_with_shift("if");
    add_keyword_with_shift("match");
    add_keyword_with_shift("loop");
    add_keyword_with_shift("while");
    add_keyword_with_shift("for");

    add_keyword_with_shift("let");

    add_keyword("true");
    add_keyword("false");
}

/// The state of the path we are currently completing.
#[derive(Debug, Default)]
pub(crate) struct PathCompletionCtx {
    /// If this is a call with () already there (or {} in case of record patterns)
    pub(crate) has_call_parens: bool,
    /// Whether the path segment has type args or not.
    pub(crate) has_type_args: bool,
    // /// The qualifier of the current path.
    // pub(crate) qualified: Qualified,
    // /// The parent of the path we are completing.
    // pub(crate) parent: Option<ast::Path>,
    // #[allow(dead_code)]
    // /// The path of which we are completing the segment
    // pub(crate) path: ast::Path,
    // /// The path of which we are completing the segment in the original file
    // pub(crate) original_path: Option<ast::Path>,
    // pub(crate) kind: PathKind,
    /// Whether the qualifier comes from a use tree parent or not
    pub(crate) has_use_stmt_parent: bool,
}

impl PathCompletionCtx {
    pub fn has_any_parens(&self) -> bool {
        self.has_call_parens || self.has_type_args
    }
}

fn path_completion_ctx(path: &InFile<ast::Path>) -> PathCompletionCtx {
    let (_, path) = path.unpack_ref();

    let ident_token = path
        .segment()
        .and_then(|it| it.name_ref())
        .map(|it| it.ident_token());

    let has_type_args = ident_token
        .clone()
        .and_then(|it| it.next_token_no_trivia())
        .is_some_and(|it| it.kind() == T![<]);
    let has_call_parens = ident_token
        .clone()
        .and_then(|it| it.next_token_no_trivia())
        .is_some_and(|it| it.kind() == T!['(']);
    let has_use_stmt_parent = path.syntax().has_ancestor_strict::<ast::UseStmt>();

    PathCompletionCtx {
        has_call_parens,
        has_type_args,
        has_use_stmt_parent,
    }
}
