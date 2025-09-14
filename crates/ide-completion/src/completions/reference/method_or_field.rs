// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::completions::Completions;
use crate::context::CompletionContext;
use crate::render::function::{FunctionKind, render_function};
use crate::render::new_named_item;
use lang::loc::SyntaxLocFileExt;
use lang::nameres::is_visible::is_visible_in_context;
use lang::nameres::path_resolution::get_method_resolve_variants;
use lang::types::has_type_params_ext::GenericItemExt;
use lang::types::inference::{InferenceCtx, TyVarIndex};
use lang::types::substitution::ApplySubstitution;
use lang::types::ty::Ty;
use lang::types::ty::adt::TyAdt;
use lang::types::{ty, ty_db};
use std::cell::RefCell;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, ast};

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

    let db = ctx.db;
    let msl = ctx.msl;

    // if we're not in the module where struct/enum are declared
    if !msl && ctx.containing_module() != ty_adt.adt_item_module(db) {
        return None;
    }

    let (file_id, adt_item) = ty_adt.adt_item_loc.to_ast::<ast::StructOrEnum>(db)?.unpack();
    let named_fields = adt_item.named_fields();
    for named_field in named_fields {
        let name = named_field.field_name().as_string();

        let mut completion_item = new_named_item(ctx, &name, named_field.syntax().kind());

        let named_field = named_field.in_file(file_id);
        if let Some(field_ty) = ty_db::lower_type_owner(db, named_field, msl) {
            let field_detail = field_ty.substitute(&ty_adt.substitution).render(db, None);
            completion_item.set_detail(Some(field_detail));
        }
        acc.add(completion_item.build(db));
    }

    Some(())
}

fn add_method_completion_items(
    completions: &RefCell<Completions>,
    ctx: &CompletionContext<'_>,
    receiver_ty: Ty,
) -> Option<()> {
    let _p = tracing::debug_span!("add_method_completion_items").entered();

    let db = ctx.db;
    let acc = &mut completions.borrow_mut();

    let original_token_ctx = InFile::new(ctx.position.file_id, ctx.original_token.clone());
    let method_entries = get_method_resolve_variants(db, &receiver_ty, ctx.position.file_id, ctx.msl)
        .into_iter()
        .filter(|e| is_visible_in_context(ctx.db, e, original_token_ctx.clone()));

    for method_entry in method_entries {
        let method_name = method_entry.name.as_str();
        let method = method_entry.cast_into::<ast::Fun>(db)?;

        let subst = method.ty_vars_subst(&TyVarIndex::default());
        let callable_ty =
            ty_db::lower_function(db, method.clone().map_into(), ctx.msl).substitute(&subst);
        let self_ty = callable_ty
            .param_types
            .first()
            .expect("all methods have self param");
        let coerced_receiver_ty =
            ty::reference::autoborrow(receiver_ty.clone(), self_ty).expect("should be compatible");

        let mut inference_ctx = InferenceCtx::new(db, method.file_id, false);
        let _ = inference_ctx.combine_types(self_ty.clone(), coerced_receiver_ty);

        let apply_subst = inference_ctx.fully_resolve_vars_fallback_to_origin(subst);
        acc.add(
            render_function(
                ctx,
                false,
                false,
                &method_name,
                method.map_into(),
                FunctionKind::Method,
                Some(apply_subst),
            )
            .build(ctx.db),
        );
    }

    Some(())
}
