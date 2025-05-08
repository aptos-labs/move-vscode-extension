pub(crate) mod name_like;

use crate::syntax_highlighting::tags::{Highlight, HlTag};
use ide_db::RootDatabase;
use lang::Semantics;
use syntax::{AstNode, AstToken, SyntaxKind, SyntaxKind::*, SyntaxNodeOrToken, SyntaxToken, T, ast};

pub(super) fn token(sema: &Semantics<'_, RootDatabase>, token: SyntaxToken) -> Option<Highlight> {
    if let Some(_comment) = ast::Comment::cast(token.clone()) {
        let h = HlTag::Comment;
        // return Some(match comment.kind().doc {
        //     Some(_) => h | HlMod::Documentation,
        //     None => h.into(),
        // });
        return Some(h.into());
    }

    let highlight: Highlight = match token.kind() {
        BYTE_STRING => HlTag::StringLiteral.into(),
        // INT_NUMBER if token.parent_ancestors().nth(1).map(|it| it.kind()) == Some(FIELD_EXPR) => {
        //     SymbolKind::Field.into()
        // }
        INT_NUMBER => HlTag::NumericLiteral.into(),
        // BYTE => HlTag::ByteLiteral.into(),
        // CHAR => HlTag::CharLiteral.into(),
        // IDENT if token.parent().and_then(ast::TokenTree::cast).is_some() => {
        //     // from this point on we are inside a token tree, this only happens for identifiers
        //     // that were not mapped down into macro invocations
        //     HlTag::None.into()
        // }
        // p if p.is_punct() => punctuation(sema, token, p),
        k if k.is_keyword() => keyword(sema, token, k)?,
        _ => return None,
    };
    Some(highlight)
}

fn keyword(
    _sema: &Semantics<'_, RootDatabase>,
    _token: SyntaxToken,
    kind: SyntaxKind,
) -> Option<Highlight> {
    let h = Highlight::new(HlTag::Keyword);
    let h = match kind {
        T![break]
        | T![continue]
        | T![else]
        | T![if]
        | T![in]
        | T![loop]
        | T![match]
        | T![return]
        | T![while]
        | T![for] => h, /*| HlMod::ControlFlow,*/
        // T![for] /*if parent_matches::<ast::ForExpr>(&token)*/ => h, /*| HlMod::ControlFlow,*/
        // T![const] if token.parent().is_some_and(|it| {
        //     matches!(
        //             it.kind(),
        //             SyntaxKind::CONST
        //                 | SyntaxKind::FUN
        //                 // | SyntaxKind::IMPL
        //                 | SyntaxKind::BLOCK_EXPR
        //                 // | SyntaxKind::CLOSURE_EXPR
        //                 // | SyntaxKind::FN_PTR_TYPE
        //                 // | SyntaxKind::TYPE_BOUND
        //                 // | SyntaxKind::CONST_BLOCK_PAT
        //         )
        // }) =>
        //     {
        //         h /*| HlMod::Const*/
        //     }
        T![true] | T![false] => HlTag::BoolLiteral.into(),
        // self, crate, super and `Self` are handled as either a Name or NameRef already, unless they
        // are inside unmapped token trees
        // T![Self] if parent_matches::<ast::NameRef>(&token) => {
        //     return None
        // }
        // T![self] if parent_matches::<ast::Name>(&token) => return None,
        _ => h,
    };
    Some(h)
}

/// Returns true if the parent nodes of `node` all match the `SyntaxKind`s in `kinds` exactly.
fn parents_match(mut node: SyntaxNodeOrToken, mut kinds: &[SyntaxKind]) -> bool {
    while let (Some(parent), [kind, rest @ ..]) = (node.parent(), kinds) {
        if parent.kind() != *kind {
            return false;
        }

        node = parent.into();
        kinds = rest;
    }

    // Only true if we matched all expected kinds
    kinds.is_empty()
}

fn parent_matches<N: AstNode>(token: &SyntaxToken) -> bool {
    token.parent().is_some_and(|it| N::can_cast(it.kind()))
}
