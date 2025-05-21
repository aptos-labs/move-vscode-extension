use crate::inlay_hints::{
    InlayHint, InlayHintPosition, InlayHintsConfig, InlayKind, label_of_ty, ty_to_text_edit,
};
use ide_db::RootDatabase;
use lang::Semantics;
use syntax::ast::NamedElement;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::files::InFile;
use syntax::{AstNode, ast};

pub(super) fn hints(
    acc: &mut Vec<InlayHint>,
    sema: &Semantics<'_, RootDatabase>,
    config: &InlayHintsConfig,
    pat: &InFile<ast::IdentPat>,
) -> Option<()> {
    if !config.type_hints {
        return None;
    }
    let ty = sema.get_ident_pat_type(pat, false)?;
    if ty.is_unknown() {
        return None;
    }

    let (file_id, pat) = pat.unpack_ref();
    // let pat = &pat.value;

    let parent = pat.syntax().parent()?.cast::<ast::IdentPatKind>()?;
    let type_ascriptable = match parent {
        ast::IdentPatKind::LambdaParam(lambda_param) => {
            if lambda_param.type_().is_some() {
                return None;
            }
            if config.hide_closure_parameter_hints {
                return None;
            }
            Some(lambda_param.colon_token())
        }
        ast::IdentPatKind::LetStmt(let_stmt) => {
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
                .map_or_else(|| pat.syntax().text_range(), |t| t.text_range())
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

    let text_range = match pat.name() {
        Some(name) => name.syntax().text_range(),
        None => pat.syntax().text_range(),
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
        resolve_parent: Some(pat.syntax().text_range()),
    });

    Some(())
}
