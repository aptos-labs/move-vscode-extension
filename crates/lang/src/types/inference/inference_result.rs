// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::loc::{SyntaxLoc, SyntaxLocNodeExt};
use crate::nameres::is_visible::ResolvedScopeEntry;
use crate::nameres::scope::{ScopeEntry, ScopeEntryListExt};
use crate::types::inference::InferenceCtx;
use crate::types::inference::combine_types::TypeError;
use crate::types::ty::Ty;
use crate::types::ty::integer::IntegerKind;
use crate::types::ty::ty_callable::TyCallable;
use crate::types::ty::ty_var::TyInfer;
use itertools::Itertools;
use std::cell::RefCell;
use std::collections::HashMap;
use syntax::{AstNode, TextRange, ast};
use vfs::FileId;

#[derive(Debug, PartialEq, Eq)]
pub struct InferenceResult {
    file_id: FileId,
    pub type_errors: Vec<TypeError>,

    pat_types: HashMap<SyntaxLoc, Ty>,
    expr_types: HashMap<SyntaxLoc, Ty>,
    call_expr_types: HashMap<SyntaxLoc, TyCallable>,

    resolved_paths: HashMap<SyntaxLoc, Vec<ResolvedScopeEntry>>,
    resolved_method_calls: HashMap<SyntaxLoc, Option<ScopeEntry>>,
    resolved_fields: HashMap<SyntaxLoc, Option<ScopeEntry>>,
    resolved_ident_pats: HashMap<SyntaxLoc, Option<ScopeEntry>>,
}

impl InferenceResult {
    pub fn from_ctx(mut ctx: InferenceCtx) -> Self {
        Self::unify_remaining_int_vars_into_integer(&mut ctx);

        let type_errors = ctx
            .type_errors
            .clone()
            .into_iter()
            .map(|type_error| ctx.fully_resolve_vars_fallback_to_origin(type_error))
            .collect::<Vec<_>>();

        let pat_types = fully_resolve_map_values(ctx.pat_types.clone(), &ctx);
        let expr_types = fully_resolve_map_values(ctx.expr_types.clone(), &ctx);

        // for call expressions, we need to leave ty vars in substitution intact to determine
        // whether an explicit type annotation required
        let call_expr_types = ctx
            .call_expr_types
            .clone()
            .into_iter()
            .map(|(any_call_expr, callable_ty)| {
                let TyCallable { param_types, ret_type, kind } = callable_ty;
                let param_tys = param_types
                    .into_iter()
                    .map(|it| ctx.fully_resolve_vars_fallback_to_origin(it))
                    .collect();
                let return_ty = ctx.fully_resolve_vars_fallback_to_origin(*ret_type);
                let res_ty =
                    TyCallable::new(param_tys, return_ty, ctx.resolve_ty_vars_if_possible(kind));
                (any_call_expr.loc(ctx.file_id), res_ty)
            })
            .collect();

        let file_id = ctx.file_id;
        let resolved_paths = keys_into_syntax_loc(ctx.resolved_paths, file_id);
        let resolved_method_calls = keys_into_syntax_loc(ctx.resolved_method_calls, file_id);
        let resolved_fields = keys_into_syntax_loc(ctx.resolved_fields, file_id);
        let resolved_ident_pats = keys_into_syntax_loc(ctx.resolved_ident_pats, file_id);

        InferenceResult {
            file_id: ctx.file_id,
            type_errors,
            pat_types,
            expr_types,
            call_expr_types,
            resolved_paths,
            resolved_method_calls,
            resolved_fields,
            resolved_ident_pats,
        }
    }

    fn unify_remaining_int_vars_into_integer(ctx: &mut InferenceCtx) {
        let int_vars = RefCell::new(vec![]);
        {
            for ty in ctx.pat_types.values().chain(ctx.expr_types.values()) {
                ty.deep_visit_ty_infers(|ty_infer| {
                    let resolved_ty_infer = ctx.resolve_ty_infer(ty_infer);
                    if let Ty::Infer(TyInfer::IntVar(int_var)) = resolved_ty_infer {
                        int_vars.borrow_mut().push(int_var);
                    }
                    false
                });
            }
        }
        for int_var in int_vars.into_inner() {
            let _ = ctx.unify_int_var(int_var, Ty::Integer(IntegerKind::Integer));
        }
    }

    pub fn get_pat_type(&self, pat_loc: &SyntaxLoc) -> Option<Ty> {
        self.pat_types.get(pat_loc).cloned()
    }

    pub fn get_expr_type(&self, expr_loc: &SyntaxLoc) -> Option<Ty> {
        self.expr_types.get(&expr_loc).cloned()
    }

    pub fn get_call_expr_type(&self, call_expr_loc: &SyntaxLoc) -> Option<TyCallable> {
        self.call_expr_types.get(call_expr_loc).cloned()
    }

    pub fn get_resolve_method_or_path(&self, method_or_path: ast::MethodOrPath) -> Option<ScopeEntry> {
        self.get_resolve_method_or_path_entries(method_or_path)
            .into_iter()
            .filter_map(|it| it.into_entry_if_visible())
            .exactly_one()
            .ok()
    }

    pub fn get_resolve_method_or_path_entries(
        &self,
        method_or_path: ast::MethodOrPath,
    ) -> Vec<ResolvedScopeEntry> {
        let loc = method_or_path.loc(self.file_id);
        match method_or_path {
            ast::MethodOrPath::MethodCallExpr(_) => {
                let resolved_entry = self.resolved_method_calls.get(&loc).cloned().unwrap_or_default();
                resolved_entry
                    .map(|e| vec![e])
                    .unwrap_or_default()
                    .into_resolved_list()
            }
            ast::MethodOrPath::Path(_) => self
                .resolved_paths
                .get(&loc)
                .map(|entries| entries.clone())
                .unwrap_or_default(),
        }
    }

    pub fn get_resolved_field(&self, field_name_ref: &ast::NameRef) -> Option<ScopeEntry> {
        let loc = field_name_ref.loc(self.file_id);
        self.resolved_fields.get(&loc).and_then(|field| field.to_owned())
    }

    pub fn get_resolved_ident_pat(&self, ident_pat: &ast::IdentPat) -> Option<ScopeEntry> {
        let loc = ident_pat.loc(self.file_id);
        self.resolved_ident_pats
            .get(&loc)
            .and_then(|ident_pat| ident_pat.to_owned())
    }

    pub fn has_type_error_inside_range(&self, range: TextRange) -> bool {
        self.type_errors
            .iter()
            .any(|it| range.contains_range(it.text_range()))
    }

    // fn get_resolved_path_entries(&self, path: &ast::Path) -> Vec<ScopeEntry> {
    //     let loc = path.loc(self.file_id);
    //     self.resolved_paths
    //         .get(&loc)
    //         .map(|entries| entries.clone())
    //         .unwrap_or_default()
    // }

    // fn get_resolved_method_call(&self, method_call_expr: &ast::MethodCallExpr) -> Option<ScopeEntry> {
    //     let loc = method_call_expr.loc(self.file_id);
    //     self.resolved_method_calls
    //         .get(&loc)
    //         .and_then(|method| method.to_owned())
    // }
}

fn fully_resolve_map_values(
    ty_map: HashMap<impl AstNode, Ty>,
    ctx: &InferenceCtx,
) -> HashMap<SyntaxLoc, Ty> {
    ty_map
        .into_iter()
        .map(|(pat, ty)| {
            let res_ty = ctx.fully_resolve_vars(ty);
            (pat.loc(ctx.file_id), res_ty)
        })
        .collect()
}

#[allow(unused)]
fn fully_resolve_map_values_fallback_to_origin(
    ty_map: HashMap<impl AstNode, Ty>,
    ctx: &InferenceCtx,
) -> HashMap<SyntaxLoc, Ty> {
    ty_map
        .into_iter()
        .map(|(pat, ty)| {
            let res_ty = ctx.fully_resolve_vars_fallback_to_origin(ty);
            (pat.loc(ctx.file_id), res_ty)
        })
        .collect()
}

fn keys_into_syntax_loc<K: AstNode, V>(map: HashMap<K, V>, file_id: FileId) -> HashMap<SyntaxLoc, V> {
    map.into_iter()
        .map(|(method_call, opt_entry)| (method_call.loc(file_id), opt_entry))
        .collect()
}
