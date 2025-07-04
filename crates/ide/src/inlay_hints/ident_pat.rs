// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::inlay_hints::{
    InlayHint, InlayHintPosition, InlayHintsConfig, InlayKind, label_of_ty, ty_to_text_edit,
};
use ide_db::RootDatabase;
use lang::Semantics;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::files::InFile;
use syntax::{AstNode, ast};

pub(super) fn hints(
    acc: &mut Vec<InlayHint>,
    sema: &Semantics<'_, RootDatabase>,
    config: &InlayHintsConfig,
    ident_pat: &InFile<ast::IdentPat>,
) -> Option<()> {
    if !config.type_hints {
        return None;
    }
    let ty = sema.get_ident_pat_type(ident_pat, false)?;
    if ty.is_unknown() {
        return None;
    }

    let (file_id, ident_pat) = ident_pat.unpack_ref();
    if ident_pat.name().is_some_and(|it| it.as_string().starts_with("_")) {
        return None;
    }

    let parent = ident_pat.syntax().parent()?.cast::<ast::IdentPatOwner>()?;
    let type_ascriptable = match parent {
        ast::IdentPatOwner::LambdaParam(lambda_param) => {
            if lambda_param.type_().is_some() {
                return None;
            }
            if config.hide_closure_parameter_hints {
                return None;
            }
            Some(lambda_param.colon_token())
        }
        ast::IdentPatOwner::LetStmt(let_stmt) => {
            if let_stmt.type_().is_some() {
                return None;
            }
            Some(let_stmt.colon_token())
        }
        _ => {
            return None;
        }
    };

    let mut label = label_of_ty(&sema, config, file_id, &ty)?;

    let text_edit = if let Some(colon_token) = &type_ascriptable {
        ty_to_text_edit(
            &sema,
            config,
            ty,
            colon_token
                .as_ref()
                .map_or_else(|| ident_pat.syntax().text_range(), |t| t.text_range())
                .end(),
            &|_| (),
            if colon_token.is_some() { "" } else { ": " },
        )
    } else {
        None
    };

    let render_colons = config.render_colons && !matches!(type_ascriptable, Some(Some(_)));
    if render_colons {
        label.prepend_str(": ");
    }

    let text_range = match ident_pat.name() {
        Some(name) => name.syntax().text_range(),
        None => ident_pat.syntax().text_range(),
    };

    acc.push(InlayHint {
        range: match type_ascriptable {
            Some(Some(t)) => text_range.cover(t.text_range()),
            _ => text_range,
        },
        kind: InlayKind::Type,
        label,
        text_edit,
        position: InlayHintPosition::After,
        pad_left: !render_colons,
        pad_right: false,
        resolve_parent: Some(ident_pat.syntax().text_range()),
    });

    Some(())
}
