// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::loc::SyntaxLocFileExt;
use crate::types::expectation::Expected;
use crate::types::inference::ast_walker::TypeAstWalker;
use crate::types::ty::Ty;
use crate::types::ty::ty_callable::{TyCallable, TyCallableKind};
use syntax::files::InFileExt;
use syntax::{IntoNodeOrToken, ast};

impl TypeAstWalker<'_, '_> {
    pub(super) fn infer_lambda_expr(&mut self, lambda_expr: &ast::LambdaExpr, expected: Expected) -> Ty {
        let mut param_tys = vec![];
        for (lambda_param, ident_pat) in lambda_expr.params_with_ident_pats() {
            let file_id = self.ctx.file_id;
            let param_ty = match lambda_param.type_() {
                Some(type_) => self.ctx.ty_lowering().lower_type(type_.in_file(file_id)),
                None => Ty::new_ty_var(&self.ctx.ty_var_index),
            };
            self.ctx.pat_types.insert(ident_pat.into(), param_ty.clone());
            param_tys.push(param_ty);
        }

        let lambda_call_ty = TyCallable::new(
            param_tys,
            Ty::new_ty_var(&self.ctx.ty_var_index),
            TyCallableKind::Lambda(Some(lambda_expr.clone().in_file(self.ctx.file_id).loc())),
        );

        // defer inference
        self.ctx.lambda_exprs.push(lambda_expr.clone());

        // need to infer return type to proceed further

        // // Eagerly try to relate the closure type with the expected
        // // type, otherwise we often won't have enough information to
        // // infer the body.
        // if let Some(t) = expected.ty(self.ctx) {
        //     dbg!(&t);
        // }

        self.ctx
            .lambda_expr_types
            .insert(lambda_expr.clone(), lambda_call_ty.clone());

        let lambda_ty = Ty::Callable(lambda_call_ty);
        if let Some(expected_ty) = expected.ty(self.ctx) {
            // error if not TyCallable
            self.ctx
                .coerce_types(lambda_expr.node_or_token(), lambda_ty.clone(), expected_ty);
        }

        lambda_ty
    }
}
