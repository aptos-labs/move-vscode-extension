// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::inlay_hints::{InlayHint, InlayHintLabel, InlayHintPosition, InlayHintsConfig, InlayKind};
use ide_db::RootDatabase;
use lang::Semantics;
use syntax::{SyntaxToken, ast};

pub(super) fn hints(
    acc: &mut Vec<InlayHint>,
    _sema: &Semantics<'_, RootDatabase>,
    config: &InlayHintsConfig,
    range_expr: ast::RangeExpr,
) -> Option<()> {
    (config.range_exclusive_hints && range_expr.end_expr().is_some())
        .then(|| {
            range_expr.dotdot_token().map(|token| {
                acc.push(inlay_hint(token));
            })
        })
        .flatten()
}

fn inlay_hint(token: SyntaxToken) -> InlayHint {
    InlayHint {
        range: token.text_range(),
        position: InlayHintPosition::After,
        pad_left: false,
        pad_right: false,
        kind: InlayKind::RangeExclusive,
        label: InlayHintLabel::from("<"),
        text_edit: None,
        resolve_parent: None,
    }
}
