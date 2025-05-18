use crate::completions::reference::paths::PathCompletionCtx;
use crate::context::CompletionContext;
use crate::item::CompletionItemBuilder;
use crate::render::render_named_item;
use base_db::SourceDatabase;
use ide_db::AllowSnippets;
use lang::types::lowering::TyLowering;
use lang::types::substitution::{ApplySubstitution, Substitution};
use lang::types::ty::Ty;
use lang::types::ty::ty_callable::TyCallable;
use syntax::ast;
use syntax::ast::NamedElement;
use syntax::files::InFile;

pub(crate) fn render_function(
    ctx: &CompletionContext<'_>,
    path_ctx: &PathCompletionCtx,
    fun_name: String,
    fun: InFile<ast::AnyFun>,
    kind: FunctionKind,
    apply_subst: Option<Substitution>,
) -> CompletionItemBuilder {
    let mut item_builder = render_named_item(ctx, fun_name, fun.clone().map_into());

    let ty_lowering = TyLowering::new(ctx.db, ctx.msl);
    let mut call_ty = ty_lowering.lower_any_function(fun.clone().map_into());
    if let Some(apply_subst) = apply_subst {
        call_ty = call_ty.substitute(&apply_subst);
    }

    let (_, fun) = fun.unpack();

    let function_name = fun.name().unwrap().as_string();
    item_builder.lookup_by(function_name.clone());

    let params = render_params(ctx.db, fun.clone(), call_ty.clone()).unwrap_or_default();
    let params = match kind {
        FunctionKind::Fun => params,
        FunctionKind::Method => params.into_iter().skip(1).collect(),
    };
    let params_line = params.join(", ");
    item_builder.set_label(format!("{function_name}({params_line})"));

    if let Some(_) = ctx.config.allow_snippets {
        let snippet_parens = if path_ctx.has_use_stmt_parent {
            "$0"
        } else {
            if params.is_empty() {
                if path_ctx.has_any_parens() { "$0" } else { "()$0" }
            } else {
                if path_ctx.has_any_parens() { "$0" } else { "($0)" }
            }
        };
        item_builder.insert_snippet(format!("{function_name}{snippet_parens}"));
    }

    match call_ty.ret_type().unwrap_all_refs() {
        Ty::Unit => (),
        ret_ty => {
            let ret_ty_txt = ret_ty.render(ctx.db, None);
            item_builder.set_detail(Some(ret_ty_txt));
        }
    }

    item_builder
}

fn render_params(db: &dyn SourceDatabase, fun: ast::AnyFun, call_ty: TyCallable) -> Option<Vec<String>> {
    let params_with_types = fun
        .params()
        .into_iter()
        .zip(call_ty.param_types.into_iter())
        .collect::<Vec<_>>();
    let mut res = vec![];
    for (param, ty) in params_with_types.into_iter() {
        let param_name = param.ident_name();
        let rendered_ty = ty.render(db, None);
        res.push(format!("{}: {}", param_name, rendered_ty));
    }
    Some(res)
}

pub(crate) enum FunctionKind {
    Fun,
    Method,
}
