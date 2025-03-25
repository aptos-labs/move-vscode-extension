use crate::completions::Completions;
use crate::context::CompletionContext;
use crate::render::function::{render_function, FunctionKind};
use crate::render::render_named_item;
use base_db::Upcast;
use lang::files::{InFileExt};
use lang::nameres::path_resolution::get_method_resolve_variants;
use lang::types::ty::adt::TyAdt;
use lang::types::ty::Ty;
use lang::InFile;
use std::cell::{RefCell, RefMut};
use syntax::ast;

pub(crate) fn add_method_or_field_completions(
    completions: &RefCell<Completions>,
    ctx: &CompletionContext<'_>,
    field_ref: InFile<ast::FieldRef>,
) -> Option<()> {
    let InFile {
        file_id,
        value: field_ref,
    } = field_ref;

    let receiver_expr = field_ref.dot_expr().receiver_expr();
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

    let method_entries = get_method_resolve_variants(hir_db, &receiver_ty);
    for method_entry in method_entries {
        let method = method_entry.cast_into::<ast::Fun>(hir_db).unwrap();

        // let subst = method.ty_vars_subst();
        // let InFile {
        //     file_id: method_file_id,
        //     value: method,
        // } = method;
        // let callable_ty = TyLowering::new(hir_db, method_file_id)
        //     .lower_function(method)
        //     .substitute(subst);
        // let self_ty = callable_ty
        //     .param_types
        //     .first()
        //     .expect("all methods have self param");
        // let coerced_receiver_ty =
        //     ty::reference::autoborrow(receiver_ty.clone(), self_ty).expect("should be compatible");
        //
        // let mut inference_ctx = InferenceCtx::new(hir_db, method_file_id);
        // let _ = inference_ctx.combine_types(self_ty.clone(), coerced_receiver_ty);

        acc.add(render_function(ctx, method.value, FunctionKind::Method).build(ctx.db));
    }

    Some(())
}

fn add_field_ref_completion_items(
    acc: &mut RefMut<Completions>,
    ctx: &CompletionContext<'_>,
    ty_adt: TyAdt,
) -> Option<()> {
    let adt_item = ty_adt
        .adt_item
        .cast_into::<ast::StructOrEnum>(ctx.db.upcast())?
        .value;
    let named_fields = adt_item.field_ref_lookup_fields();
    for named_field in named_fields {
        acc.add(render_named_item(ctx, named_field.into()).build(ctx.db));
    }

    Some(())
}
