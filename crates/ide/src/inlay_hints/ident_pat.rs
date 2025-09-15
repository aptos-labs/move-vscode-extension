// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::inlay_hints::{InlayHint, InlayHintPosition, InlayHintsConfig, InlayKind, label_of_ty};
use ide_db::RootDatabase;
use lang::Semantics;
use syntax::ast::node_ext::syntax_element::SyntaxElementExt;
use syntax::files::InFile;
use syntax::{AstNode, ast};

pub(super) fn hints(
    acc: &mut Vec<InlayHint>,
    sema: &Semantics<'_, RootDatabase>,
    config: &InlayHintsConfig,
    ident_pat: InFile<ast::IdentPat>,
) -> Option<()> {
    if !config.type_hints {
        return None;
    }
    let ty = sema.get_ident_pat_type(&ident_pat, false)?;
    if ty.is_unknown() {
        return None;
    }

    let (file_id, ident_pat) = ident_pat.unpack();
    if ident_pat.name().is_some_and(|it| it.as_string().starts_with("_")) {
        return None;
    }

    let ident_pat_owner = ident_pat.ident_owner()?;
    let colon_token = match ident_pat_owner {
        ast::IdentPatOwner::LambdaParam(lambda_param) => {
            if lambda_param.type_().is_some() {
                return None;
            }
            if config.hide_closure_parameter_hints {
                return None;
            }
            lambda_param.colon_token()
        }
        ast::IdentPatOwner::LetStmt(let_stmt) => {
            if let_stmt.type_().is_some() {
                return None;
            }
            if !config.tuple_type_hints && ident_pat.syntax().parent_is::<ast::TuplePat>() {
                return None;
            }
            let_stmt.colon_token()
        }
        _ => {
            return None;
        }
    };

    let mut label = label_of_ty(&sema, config, file_id, &ty)?;

    let render_colons = config.render_colons && !matches!(colon_token, Some(_));
    if render_colons {
        label.prepend_str(": ");
    }

    let text_range = match ident_pat.name() {
        Some(name) => name.syntax().text_range(),
        None => ident_pat.syntax().text_range(),
    };

    acc.push(InlayHint {
        range: match colon_token {
            Some(t) => text_range.cover(t.text_range()),
            _ => text_range,
        },
        kind: InlayKind::Type,
        label,
        text_edit: None,
        position: InlayHintPosition::After,
        pad_left: !render_colons,
        pad_right: false,
        resolve_parent: Some(ident_pat.syntax().text_range()),
    });

    Some(())
}
