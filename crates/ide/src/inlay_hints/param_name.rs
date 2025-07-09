// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::inlay_hints::{InlayHint, InlayHintLabel, InlayHintPosition, InlayHintsConfig, InlayKind};
use ide_db::RootDatabase;
use lang::Semantics;
use lang::types::ty::ty_callable::Callable;
use syntax::files::InFile;
use syntax::{AstNode, ast};

pub(super) fn hints(
    acc: &mut Vec<InlayHint>,
    sema: &Semantics<'_, RootDatabase>,
    config: &InlayHintsConfig,
    call_expr: InFile<ast::AnyCallExpr>,
) -> Option<()> {
    if !config.parameter_hints {
        return None;
    }

    let callable_ty = sema.get_call_expr_type(&call_expr)?;
    let arg_exprs = call_expr.value.arg_exprs();

    let callable = callable_ty.kind.callable(sema.db)?;
    let params = match callable {
        Callable::Fun(any_fun) => {
            let params = any_fun
                .value
                .params()
                .iter()
                .map(|it| Some(it.ident_name()))
                .collect::<Vec<_>>();
            params
        }
        Callable::LambdaExpr(lambda_expr) => {
            let params = lambda_expr
                .value
                .params()
                .iter()
                .map(|it| it.ident_pat()?.name().map(|it| it.as_string()))
                .collect::<Vec<_>>();
            params
        }
    };

    for (param, arg_expr) in params.iter().zip(arg_exprs) {
        if let (Some(param), Some(arg_expr)) = (param, arg_expr) {
            if matches!(arg_expr, ast::Expr::Literal(_) | ast::Expr::BinExpr(_)) {
                let mut label = InlayHintLabel::simple(param, None, None);
                if config.render_colons {
                    label.append_str(": ");
                }
                acc.push(InlayHint {
                    range: arg_expr.syntax().text_range(),
                    kind: InlayKind::Parameter,
                    label,
                    text_edit: None,
                    position: InlayHintPosition::Before,
                    pad_left: false,
                    pad_right: true,
                    resolve_parent: None,
                });
            }
        }
    }

    Some(())
}
