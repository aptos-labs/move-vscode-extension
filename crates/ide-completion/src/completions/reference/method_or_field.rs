use crate::completions::Completions;
use crate::context::CompletionContext;
use crate::render::function::{FunctionKind, render_function};
use crate::render::render_named_item;
use base_db::Upcast;
use lang::InFile;
use lang::files::{InFileExt, InFileInto};
use lang::nameres::path_resolution::get_method_resolve_variants;
use lang::types::has_type_params_ext::GenericItemExt;
use lang::types::inference::InferenceCtx;
use lang::types::lowering::TyLowering;
use lang::types::substitution::ApplySubstitution;
use lang::types::ty;
use lang::types::ty::Ty;
use lang::types::ty::adt::TyAdt;
use std::cell::{RefCell, RefMut};
use syntax::ast;

pub(crate) fn add_method_or_field_completions(
    completions: &RefCell<Completions>,
    ctx: &CompletionContext<'_>,
    receiver_expr: InFile<ast::Expr>,
) -> Option<()> {
    let (file_id, receiver_expr) = receiver_expr.unpack();

    let inference = receiver_expr
        .inference_ctx_owner()?
        .in_file(file_id)
        .inference(ctx.db.upcast());

    let receiver_ty = inference.get_expr_type(&receiver_expr)?;
    if receiver_ty.deref() == Ty::Unknown {
        return None;
    }

    let acc = &mut completions.borrow_mut();

    if let Some(ty_adt) = receiver_ty.deref().into_ty_adt() {
        add_field_ref_completion_items(acc, ctx, ty_adt);
    }

    let hir_db = ctx.db.upcast();

    let method_entries = get_method_resolve_variants(hir_db, &receiver_ty, file_id);
    for method_entry in method_entries {
        let method = method_entry.cast_into::<ast::Fun>(hir_db).unwrap();

        let subst = method.ty_vars_subst();
        let callable_ty = TyLowering::new(hir_db)
            .lower_function(method.clone())
            .substitute(&subst);
        let self_ty = callable_ty
            .param_types
            .first()
            .expect("all methods have self param");
        let coerced_receiver_ty =
            ty::reference::autoborrow(receiver_ty.clone(), self_ty).expect("should be compatible");

        let mut inference_ctx = InferenceCtx::new(hir_db, method.file_id);
        let _ = inference_ctx.combine_types(self_ty.clone(), coerced_receiver_ty);

        let apply_subst = inference_ctx.fully_resolve_vars_fallback_to_origin(subst);
        acc.add(render_function(ctx, method, FunctionKind::Method, Some(apply_subst)).build(ctx.db));
    }

    Some(())
}

fn add_field_ref_completion_items(
    acc: &mut RefMut<Completions>,
    ctx: &CompletionContext<'_>,
    ty_adt: TyAdt,
) -> Option<()> {
    let (file_id, adt_item) = ty_adt
        .adt_item
        .into_ast::<ast::StructOrEnum>(ctx.db.upcast())?
        .unpack();
    let named_fields = adt_item.field_ref_lookup_fields();
    let ty_lowering = TyLowering::new(ctx.db);
    for named_field in named_fields {
        let named_field = named_field.in_file(file_id);
        let mut completion_item = render_named_item(ctx, named_field.clone().in_file_into());
        if let Some(field_type) = named_field.and_then(|it| it.type_()) {
            let field_ty = ty_lowering.lower_type(field_type);
            let field_detail = field_ty.substitute(&ty_adt.substitution).render(ctx.db.upcast());
            completion_item.set_detail(Some(field_detail));
        }
        acc.add(completion_item.build(ctx.db));
    }

    Some(())
}
