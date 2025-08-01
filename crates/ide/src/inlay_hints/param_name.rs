// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::inlay_hints::{InlayHint, InlayHintLabel, InlayHintPosition, InlayHintsConfig, InlayKind};
use ide_db::RootDatabase;
use ide_db::defs::BUILTIN_MUT_RESOURCE_FUNCTIONS;
use lang::Semantics;
use std::collections::HashSet;
use std::sync::LazyLock;
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
    if callable
        .callable_item
        .name()
        .is_some_and(|name| KNOWN_CALLABLES.contains(name.as_str()))
    {
        return None;
    }

    let params = callable.params()?;
    // skip if only a single parameter
    if params.len() == 1 {
        return None;
    }

    let arg_exprs = call_expr.value.arg_exprs();
    for (param_name, arg_expr) in params.into_iter().map(|it| it.name).zip(arg_exprs) {
        if let (Some(param_name), Some(arg_expr)) = (param_name, arg_expr) {
            // leading underscore is ignored
            let param_name = param_name.trim_start_matches("_");
            let arg_expr = arg_expr.syntax();
            if arg_expr
                .text()
                .to_string()
                .to_lowercase()
                .contains(&param_name.to_lowercase())
            {
                continue;
            }
            let mut label = InlayHintLabel::simple(param_name, None, None);
            if config.render_colons {
                label.append_str(": ");
            }
            acc.push(InlayHint {
                range: arg_expr.text_range(),
                kind: InlayKind::Parameter,
                label,
                text_edit: None,
                position: InlayHintPosition::Before,
                pad_left: false,
                pad_right: false,
                resolve_parent: None,
            });
        }
    }

    Some(())
}

static KNOWN_CALLABLES: LazyLock<HashSet<&str>> = LazyLock::new(|| {
    let mut res = HashSet::new();
    res.extend(BUILTIN_MUT_RESOURCE_FUNCTIONS.clone());
    res.insert("assert!");
    res.insert("exists");
    res
});
