use crate::completions::Completions;
use crate::context::CompletionContext;
use crate::item::CompletionItem;
use crate::render::function::{render_function, FunctionKind};
use crate::render::render_named_item;
use base_db::Upcast;
use lang::nameres::path_kind::path_kind;
use lang::nameres::path_resolution::{get_path_resolve_variants, ResolutionContext};
use lang::nameres::scope::ScopeEntryListExt;
use lang::InFile;
use std::cell::RefCell;
use syntax::{ast, AstNode};

pub(crate) fn add_path_completions(
    completions: &RefCell<Completions>,
    ctx: &CompletionContext<'_>,
    path: InFile<ast::Path>,
) -> Option<()> {
    let path_kind = path_kind(path.clone(), true);
    tracing::debug!(path_kind = ?path_kind);

    if path_kind.is_unqualified() {
        add_keywords(completions, ctx);
    }

    let acc = &mut completions.borrow_mut();

    let resolution_ctx = ResolutionContext {
        path: path.clone(),
        is_completion: true,
    };
    let entries = get_path_resolve_variants(ctx.db.upcast(), &resolution_ctx, path_kind);
    tracing::debug!(entries = ?entries);

    let filtered_entries = entries.filter_by_visibility(ctx.db.upcast(), &path);
    tracing::debug!(filtered_entries = ?filtered_entries);

    for entry in filtered_entries {
        let named_item = entry
            .cast_into::<ast::AnyNamedElement>(ctx.db.upcast())
            .unwrap()
            .value;
        if let Some(function) = named_item.cast_into::<ast::Fun>() {
            acc.add(render_function(ctx, function, FunctionKind::Fun).build(ctx.db));
            return Some(());
        }
        acc.add(render_named_item(ctx, named_item).build(ctx.db));
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
