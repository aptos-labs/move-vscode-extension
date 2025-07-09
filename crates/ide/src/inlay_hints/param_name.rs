// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::inlay_hints::{InlayHint, InlayHintLabel, InlayHintPosition, InlayHintsConfig, InlayKind};
use ide_db::RootDatabase;
use lang::Semantics;
use lang::node_ext::callable::CallableItem;
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

    let callable = sema.callable(&call_expr)?;
    if matches!(callable.callable_item, CallableItem::AssertMacro) {
        return None;
    }

    let params = callable.params()?;
    let arg_exprs = call_expr.value.arg_exprs();
    for (param_name, arg_expr) in params.into_iter().map(|it| it.name).zip(arg_exprs) {
        if let (Some(param_name), Some(arg_expr)) = (param_name, arg_expr) {
            if matches!(arg_expr, ast::Expr::Literal(_) | ast::Expr::BinExpr(_)) {
                let mut label = InlayHintLabel::simple(param_name, None, None);
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
