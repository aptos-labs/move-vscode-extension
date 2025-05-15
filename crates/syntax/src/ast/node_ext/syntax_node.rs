use crate::{ast, AstNode, AstToken, SyntaxElement, SyntaxNode, SyntaxToken, TextRange, TextSize};
use parser::SyntaxKind;
use rowan::{Direction, NodeOrToken, TokenAtOffset};
use std::cmp::Ordering;
use stdx::itertools::Itertools;

pub trait SyntaxElementExt {
    fn prev_sibling_or_token_no_trivia(&self) -> Option<SyntaxElement>;
}

impl SyntaxElementExt for SyntaxElement {
    fn prev_sibling_or_token_no_trivia(&self) -> Option<SyntaxElement> {
        match self {
            NodeOrToken::Node(node) => node
                .siblings_with_tokens(Direction::Prev)
                .dropping(1)
                .filter(|it| !it.kind().is_trivia())
                .next(),
            NodeOrToken::Token(token) => token
                .siblings_with_tokens(Direction::Prev)
                .dropping(1)
                .filter(|it| !it.kind().is_trivia())
                .next(),
        }
    }
}

pub trait SyntaxNodeExt {
    fn token_at_offset_exact(&self, offset: TextSize) -> Option<SyntaxToken>;
    fn ident_at_offset(&self, offset: TextSize) -> Option<ast::Ident>;

    fn is_ancestor_of(&self, node: &SyntaxNode) -> bool;

    fn ancestor_of_kind(&self, kind: SyntaxKind, strict: bool) -> Option<SyntaxNode>;
    fn ancestor_strict_of_kind(&self, kind: SyntaxKind) -> Option<SyntaxNode>;

    fn ancestor_of_type<Ast: AstNode>(&self, strict: bool) -> Option<Ast>;
    fn ancestor_or_self<Ast: AstNode>(&self) -> Option<Ast>;
    fn ancestor_strict<Ast: AstNode>(&self) -> Option<Ast>;

    fn has_ancestor_strict<Ast: AstNode>(&self) -> bool;

    fn parent_of_type<Ast: AstNode>(&self) -> Option<Ast>;

    fn descendants_of_type<Ast: AstNode>(&self) -> impl Iterator<Item = Ast>;

    fn next_sibling_or_token_no_trivia(&self) -> Option<SyntaxElement>;

    fn strictly_before(&self, other: &SyntaxNode) -> bool;
}

impl SyntaxNodeExt for SyntaxNode {
    fn token_at_offset_exact(&self, offset: TextSize) -> Option<SyntaxToken> {
        let token_at_offset = self.token_at_offset(offset);
        match token_at_offset {
            TokenAtOffset::None => None,
            TokenAtOffset::Single(token) => Some(token),
            TokenAtOffset::Between(_, _) => None,
        }
    }

    fn ident_at_offset(&self, offset: TextSize) -> Option<ast::Ident> {
        let token = self.token_at_offset_exact(offset)?;
        ast::Ident::cast(token)
    }

    fn is_ancestor_of(&self, node: &SyntaxNode) -> bool {
        node.ancestors().any(|it| &it == self)
    }

    fn ancestor_of_kind(&self, kind: SyntaxKind, strict: bool) -> Option<SyntaxNode> {
        if !strict && self.kind() == kind {
            return Some(self.to_owned());
        }
        self.ancestors().find(|ans| ans.kind() == kind)
    }

    fn ancestor_strict_of_kind(&self, kind: SyntaxKind) -> Option<SyntaxNode> {
        self.ancestor_of_kind(kind, true)
    }

    fn ancestor_of_type<Ast: AstNode>(&self, strict: bool) -> Option<Ast> {
        if !strict && Ast::can_cast(self.kind()) {
            return Ast::cast(self.to_owned());
        }
        self.ancestors().find_map(Ast::cast)
    }

    fn ancestor_or_self<Ast: AstNode>(&self) -> Option<Ast> {
        self.ancestor_of_type(false)
    }

    fn ancestor_strict<Ast: AstNode>(&self) -> Option<Ast> {
        self.ancestor_of_type(true)
    }

    fn has_ancestor_strict<Ast: AstNode>(&self) -> bool {
        self.ancestor_strict::<Ast>().is_some()
    }

    fn parent_of_type<Ast: AstNode>(&self) -> Option<Ast> {
        let parent_node = self.parent()?;
        Ast::cast(parent_node)
    }

    fn descendants_of_type<Ast: AstNode>(&self) -> impl Iterator<Item = Ast> {
        self.descendants().filter_map(Ast::cast)
    }

    fn next_sibling_or_token_no_trivia(&self) -> Option<SyntaxElement> {
        self.siblings_with_tokens(Direction::Next)
            .skip(1)
            .filter(|it| !it.kind().is_trivia())
            .next()
    }

    fn strictly_before(&self, other: &SyntaxNode) -> bool {
        let left_range = self.text_range();
        let right_range = other.text_range();
        left_range.ordering(right_range) == Ordering::Less
    }
}

pub trait SyntaxTokenExt {
    fn is(&self, kind: SyntaxKind) -> bool;
}

impl SyntaxTokenExt for SyntaxToken {
    fn is(&self, kind: SyntaxKind) -> bool {
        self.kind() == kind
    }
}
