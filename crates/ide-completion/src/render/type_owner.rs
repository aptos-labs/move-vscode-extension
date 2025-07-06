use crate::context::CompletionContext;
use crate::item::{CompletionItemBuilder, CompletionRelevance};
use crate::render::function::render_ty;
use crate::render::{compute_type_match, new_named_item};
use lang::types::lowering::TyLowering;
use syntax::ast;
use syntax::files::InFile;

pub(crate) fn render_ident_pat(
    ctx: &CompletionContext<'_>,
    item_name: &String,
    ident_pat: InFile<ast::IdentPat>,
) -> CompletionItemBuilder {
    let mut item = new_named_item(ctx, item_name, ident_pat.kind());
    item.with_relevance(|r| CompletionRelevance { is_local: true, ..r });

    let ident_ty = ctx.sema.get_ident_pat_type(&ident_pat, ctx.msl);
    if let Some(ident_ty) = ident_ty {
        item.set_detail(Some(render_ty(ctx, &ident_ty)));
        item.with_relevance(|r| CompletionRelevance {
            type_match: compute_type_match(ctx, ident_ty),
            ..r
        });
    }

    item
}

pub(crate) fn render_type_owner(
    ctx: &CompletionContext<'_>,
    item_name: &String,
    type_owner: InFile<ast::TypeOwner>,
) -> CompletionItemBuilder {
    let mut item = new_named_item(ctx, &item_name, type_owner.kind());
    if let Some(item_type) = type_owner.and_then(|it| it.type_()) {
        // todo: apply subst from struct / schema
        let ty = TyLowering::new(ctx.db, ctx.msl).lower_type(item_type);
        item.set_detail(Some(render_ty(ctx, &ty)));
    }
    item
}
