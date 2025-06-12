pub(crate) mod name_like;

use crate::syntax_highlighting::tags::{Highlight, HlOperator, HlPunct, HlTag};
use ide_db::SymbolKind;
use syntax::{
    AstNode, AstToken, SyntaxKind, SyntaxKind::*, SyntaxNode, SyntaxNodeOrToken, SyntaxToken, T, ast,
};

pub(super) fn token(token: SyntaxToken) -> Option<Highlight> {
    if let Some(_comment) = ast::Comment::cast(token.clone()) {
        return Some(HlTag::Comment.into());
    }

    let token_parent = token.parent().map(|it| it.kind());
    let highlight: Highlight = match token.kind() {
        BYTE_STRING => HlTag::StringLiteral.into(),
        HEX_STRING => HlTag::StringLiteral.into(),
        // INT_NUMBER if token.parent_ancestors().nth(1).map(|it| it.kind()) == Some(FIELD_EXPR) => {
        //     SymbolKind::Field.into()
        // }
        INT_NUMBER => HlTag::NumericLiteral.into(),
        IDENT if matches!(token_parent, Some(VECTOR_LIT_EXPR)) => {
            Highlight::new(HlTag::Symbol(SymbolKind::Vector))
        }
        IDENT if matches!(token_parent, Some(ASSERT_MACRO_EXPR)) => {
            Highlight::new(HlTag::Symbol(SymbolKind::Assert))
        }
        p if p.is_punct() => punctuation(token, p),
        k if k.is_keyword() => keyword(k)?,
        _ => return None,
    };
    Some(highlight)
}

fn keyword(kind: SyntaxKind) -> Option<Highlight> {
    let tag = match kind {
        T![true] | T![false] => HlTag::BoolLiteral.into(),
        _ => HlTag::Keyword,
    };
    Some(Highlight::new(tag))
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

fn punctuation(token: SyntaxToken, kind: SyntaxKind) -> Highlight {
    let operator_parent = token.parent();
    let parent_kind = operator_parent.as_ref().map_or(EOF, SyntaxNode::kind);

    match (kind, parent_kind) {
        (T![&], BIN_EXPR) => HlOperator::Bitwise.into(),
        (T![&], BORROW_EXPR) => HlTag::Operator(HlOperator::Other).into(),
        (T![..], _) => HlOperator::Other.into(),
        (T![::] | T![=>] | T![=] | T![@] | T![.], _) => HlOperator::Other.into(),
        (T![!], ASSERT_MACRO_EXPR) => HlPunct::MacroBang.into(),
        (T![!], BANG_EXPR) => HlOperator::Logical.into(),
        (T![*], DEREF_EXPR) => HlTag::Operator(HlOperator::Other).into(),
        (
            T![+] | T![-] | T![*] | T![/] | T![%] | T![+=] | T![-=] | T![*=] | T![/=] | T![%=],
            BIN_EXPR,
        ) => HlOperator::Arithmetic.into(),
        (
            T![|] | T![&] | T![^] | T![>>] | T![<<] | T![|=] | T![&=] | T![^=] | T![>>=] | T![<<=],
            BIN_EXPR,
        ) => HlOperator::Bitwise.into(),
        (T![&&] | T![||] | T![==>] | T![<==>], BIN_EXPR) => HlOperator::Logical.into(),
        (T![>] | T![<] | T![==] | T![>=] | T![<=] | T![!=], BIN_EXPR) => HlOperator::Comparison.into(),
        (_, ATTR) => HlTag::AttributeBracket.into(),
        (kind, _) => match kind {
            T!['['] | T![']'] => HlPunct::Bracket,
            T!['{'] | T!['}'] => HlPunct::Brace,
            T!['('] | T![')'] => HlPunct::Parenthesis,
            T![<] | T![>] => HlPunct::Angle,
            T![,] => HlPunct::Comma,
            T![:] => HlPunct::Colon,
            T![;] => HlPunct::Semi,
            T![.] => HlPunct::Dot,
            _ => HlPunct::Other,
        }
        .into(),
    }
}
