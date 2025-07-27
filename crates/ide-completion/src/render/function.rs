// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::context::CompletionContext;
use crate::item::{CompletionItemBuilder, CompletionRelevance};
use crate::render::{compute_type_match, new_named_item};
use lang::types::lowering::TyLowering;
use lang::types::substitution::{ApplySubstitution, Substitution};
use lang::types::ty::Ty;
use lang::types::ty::ty_callable::TyCallable;
use syntax::ast;
use syntax::files::InFile;

pub(crate) fn render_function(
    ctx: &CompletionContext<'_>,
    is_use_stmt: bool,
    has_any_parens: bool,
    fun_name: String,
    fun: InFile<ast::AnyFun>,
    kind: FunctionKind,
    apply_subst: Option<Substitution>,
) -> CompletionItemBuilder {
    let mut item_builder = new_named_item(ctx, &fun_name, fun.kind());

    let ty_lowering = TyLowering::new(ctx.db, ctx.msl);
    let mut call_ty = ty_lowering.lower_any_function(fun.clone().map_into());
    if let Some(apply_subst) = apply_subst {
        call_ty = call_ty.substitute(&apply_subst);
    }

    let (_, fun) = fun.unpack();

    let function_name = fun.name().unwrap().as_string();
    item_builder.lookup_by(function_name.clone());

    let params = render_params(ctx, fun.clone(), call_ty.clone()).unwrap_or_default();
    let params = match kind {
        FunctionKind::Fun => params,
        FunctionKind::Method => params.into_iter().skip(1).collect(),
    };
    let params_line = params.join(", ");
    item_builder.set_label(format!("{function_name}({params_line})"));

    if let Some(_) = ctx.config.allow_snippets {
        let snippet_parens = if is_use_stmt {
            "$0"
        } else {
            if params.is_empty() {
                if has_any_parens { "$0" } else { "()$0" }
            } else {
                if has_any_parens { "$0" } else { "($0)" }
            }
        };
        item_builder.insert_snippet(format!("{function_name}{snippet_parens}"));
    }

    let ret_type = call_ty.ret_type();
    match &ret_type {
        Ty::Unit => (),
        ret_ty => {
            item_builder.set_detail(Some(render_ty(ctx, ret_ty)));
        }
    }

    item_builder.with_relevance(|r| CompletionRelevance {
        type_match: compute_type_match(ctx, ret_type),
        ..r
    });

    item_builder
}

fn render_params(
    ctx: &CompletionContext<'_>,
    fun: ast::AnyFun,
    call_ty: TyCallable,
) -> Option<Vec<String>> {
    let params_with_types = fun
        .params()
        .into_iter()
        .zip(call_ty.param_types.into_iter())
        .collect::<Vec<_>>();
    let mut res = vec![];
    for (param, ty) in params_with_types.into_iter() {
        res.push(format!("{}: {}", param.ident_name(), render_ty(ctx, &ty)));
    }
    Some(res)
}

pub(crate) fn render_ty(ctx: &CompletionContext<'_>, ty: &Ty) -> String {
    ctx.sema.render_ty_for_ui(ty, ctx.position.file_id)
}

pub(crate) enum FunctionKind {
    Fun,
    Method,
}
