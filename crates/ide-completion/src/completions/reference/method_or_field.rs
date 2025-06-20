use crate::completions::Completions;
use crate::completions::reference::paths::PathCompletionCtx;
use crate::context::CompletionContext;
use crate::render::function::{FunctionKind, render_function};
use crate::render::render_named_item;
use lang::loc::SyntaxLocFileExt;
use lang::nameres::path_resolution::get_method_resolve_variants;
use lang::types::has_type_params_ext::GenericItemExt;
use lang::types::inference::{InferenceCtx, TyVarIndex};
use lang::types::lowering::TyLowering;
use lang::types::substitution::ApplySubstitution;
use lang::types::ty;
use lang::types::ty::Ty;
use lang::types::ty::adt::TyAdt;
use std::cell::RefCell;
use syntax::ast;
use syntax::files::{InFile, InFileExt};

pub(crate) fn add_method_or_field_completions(
    completions: &RefCell<Completions>,
    ctx: &CompletionContext<'_>,
    receiver_expr: InFile<ast::Expr>,
) -> Option<()> {
    let inference = ctx.sema.inference(&receiver_expr, ctx.msl)?;

    let receiver_ty = inference.get_expr_type(&receiver_expr.loc())?;
    if receiver_ty.unwrap_all_refs() == Ty::Unknown {
        return None;
    }

    if let Some(ty_adt) = receiver_ty.unwrap_all_refs().into_ty_adt() {
        add_field_completion_items(completions, ctx, ty_adt);
    }

    add_method_completion_items(completions, ctx, receiver_ty);

    Some(())
}

#[tracing::instrument(level = "debug", skip_all)]
fn add_field_completion_items(
    completions: &RefCell<Completions>,
    ctx: &CompletionContext<'_>,
    ty_adt: TyAdt,
) -> Option<()> {
    let acc = &mut completions.borrow_mut();

    // if we're not in the module where struct/enum are declared
    if !ctx.msl && ctx.containing_module() != ty_adt.adt_item_module(ctx.db) {
        return None;
    }

    let (file_id, adt_item) = ty_adt.adt_item_loc.to_ast::<ast::StructOrEnum>(ctx.db)?.unpack();
    let named_fields = adt_item.named_fields();
    let ty_lowering = TyLowering::new(ctx.db, ctx.msl);
    for named_field in named_fields {
        let name = named_field.field_name().as_string();
        let named_field = named_field.in_file(file_id);
        let mut completion_item = render_named_item(ctx, name, named_field.clone().map_into());

        if let Some(field_ty) = ty_lowering.lower_type_owner(named_field.map_into()) {
            let field_detail = field_ty.substitute(&ty_adt.substitution).render(ctx.db, None);
            completion_item.set_detail(Some(field_detail));
        }
        acc.add(completion_item.build(ctx.db));
    }

    Some(())
}

#[tracing::instrument(level = "debug", skip_all)]
fn add_method_completion_items(
    completions: &RefCell<Completions>,
    ctx: &CompletionContext<'_>,
    receiver_ty: Ty,
) -> Option<()> {
    let hir_db = ctx.db;
    let acc = &mut completions.borrow_mut();

    let method_entries =
        get_method_resolve_variants(hir_db, &receiver_ty, ctx.position.file_id, ctx.msl);
    for method_entry in method_entries {
        let method_name = method_entry.name.clone();
        let method = method_entry.cast_into::<ast::Fun>(hir_db)?;

        let subst = method.ty_vars_subst(&TyVarIndex::default());
        let callable_ty = TyLowering::new(hir_db, ctx.msl)
            .lower_any_function(method.clone().map_into())
            .substitute(&subst);
        let self_ty = callable_ty
            .param_types
            .first()
            .expect("all methods have self param");
        let coerced_receiver_ty =
            ty::reference::autoborrow(receiver_ty.clone(), self_ty).expect("should be compatible");

        let mut inference_ctx = InferenceCtx::new(hir_db, method.file_id, false);
        let _ = inference_ctx.combine_types(self_ty.clone(), coerced_receiver_ty);

        let apply_subst = inference_ctx.fully_resolve_vars_fallback_to_origin(subst);
        let path_ctx = PathCompletionCtx::default();
        acc.add(
            render_function(
                ctx,
                &path_ctx,
                method_name,
                method.map_into(),
                FunctionKind::Method,
                Some(apply_subst),
            )
            .build(ctx.db),
        );
    }

    Some(())
}
