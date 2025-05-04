use crate::completions::Completions;
use crate::context::CompletionContext;
use crate::render::function::{FunctionKind, render_function};
use crate::render::render_named_item;
use lang::nameres::path_kind::path_kind;
use lang::nameres::path_resolution::{ResolutionContext, get_path_resolve_variants};
use lang::nameres::scope::ScopeEntryListExt;
use std::cell::RefCell;
use syntax::SyntaxKind::*;
use syntax::ast;
use syntax::files::InFile;

pub(crate) fn add_path_completions(
    completions: &RefCell<Completions>,
    ctx: &CompletionContext<'_>,
    context_path: InFile<ast::Path>,
) -> Option<()> {
    let path_kind = path_kind(context_path.clone().value, true)?;
    tracing::debug!(path_kind = ?path_kind);

    if path_kind.is_unqualified() {
        add_keywords(completions, ctx);
    }

    let acc = &mut completions.borrow_mut();

    let resolution_ctx = ResolutionContext {
        path: context_path.clone(),
        is_completion: true,
    };
    let entries = get_path_resolve_variants(ctx.db, &resolution_ctx, path_kind)
        .filter_by_visibility(ctx.db, &context_path);
    tracing::debug!(?entries);

    for entry in entries {
        let named_item = entry.cast_into::<ast::AnyNamedElement>(ctx.db)?;
        match named_item.kind() {
            FUN | SPEC_FUN | SPEC_INLINE_FUN => {
                acc.add(
                    render_function(
                        ctx,
                        named_item.cast_into::<ast::AnyFun>()?,
                        FunctionKind::Fun,
                        None,
                    )
                    .build(ctx.db),
                );
                continue;
            }
            _ => {
                acc.add(render_named_item(ctx, named_item).build(ctx.db));
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
