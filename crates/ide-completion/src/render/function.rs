use crate::context::CompletionContext;
use crate::item::CompletionItemBuilder;
use crate::render::render_named_item;
use base_db::{SourceRootDatabase, Upcast};
use lang::InFile;
use lang::files::InFileInto;
use lang::types::lowering::TyLowering;
use lang::types::substitution::{ApplySubstitution, Substitution};
use lang::types::ty::Ty;
use lang::types::ty::ty_callable::TyCallable;
use syntax::ast::NamedElement;
use syntax::{AstNode, ast};

pub(crate) fn render_function(
    ctx: &CompletionContext<'_>,
    function: InFile<ast::Fun>,
    kind: FunctionKind,
    apply_subst: Option<Substitution>,
) -> CompletionItemBuilder {
    let mut completion_item = render_named_item(ctx, function.clone().in_file_into());

    let ty_lowering = TyLowering::new(ctx.db);
    let mut call_ty = ty_lowering.lower_function(function.clone());
    if let Some(apply_subst) = apply_subst {
        call_ty = call_ty.substitute(&apply_subst);
    }

    let (_, fun) = function.unpack();

    let function_name = fun.name().unwrap().as_string();
    completion_item.lookup_by(function_name.clone());

    let params = render_params(ctx.db.upcast(), fun.clone(), call_ty.clone()).unwrap_or_default();
    let params = match kind {
        FunctionKind::Fun => params,
        FunctionKind::Method => params.into_iter().skip(1).collect(),
    };

    if let Some(cap) = ctx.config.snippet_cap {
        let (snippet, label_suffix) = if params.is_empty() {
            (format!("{}()$0", &function_name), "()".to_string())
        } else {
            let params_line = params.join(", ");
            (format!("{}($0)", &function_name), format!("({})", params_line))
        };
        completion_item.set_label(format!("{}{}", &function_name, label_suffix));
        completion_item.insert_snippet(cap, snippet);
    }

    match call_ty.ret_type.deref_all() {
        Ty::Unit => (),
        ret_ty => {
            let ret_ty_txt = ret_ty.render(ctx.db.upcast());
            completion_item.set_detail(Some(ret_ty_txt));
        }
    }

    completion_item
}

fn render_params(
    db: &dyn SourceRootDatabase,
    fun: ast::Fun,
    call_ty: TyCallable,
) -> Option<Vec<String>> {
    let params_with_types = fun
        .params()
        .into_iter()
        .zip(call_ty.param_types.into_iter())
        .collect::<Vec<_>>();
    let mut res = vec![];
    for (param, ty) in params_with_types.into_iter() {
        let param_name = param.ident_pat().name()?.as_string();
        let rendered_ty = ty.render(db);
        res.push(format!("{}: {}", param_name, rendered_ty));
    }
    Some(res)
}

pub(crate) enum FunctionKind {
    Fun,
    Method,
}
