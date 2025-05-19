use crate::parse::SyntaxKind;
use crate::parse::SyntaxKind::SOURCE_FILE;
use crate::{AstNode, SourceFile, SyntaxElement, SyntaxNode, SyntaxToken, TextRange, TextSize};
use itertools::Itertools;
use rowan::{Direction, NodeOrToken};

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum ComparePos {
    Before,
    Equal,
    After,
}

pub fn compare_by_position(first: &SyntaxNode, second: &SyntaxNode) -> ComparePos {
    fn as_isize(s: TextSize) -> isize {
        u32::from(s) as isize
    }

    let first_range = first.text_range();
    let second_range = second.text_range();

    let mut result = as_isize(first_range.start()) - as_isize(second_range.start());
    if result == 0 {
        result = as_isize(first_range.end()) - as_isize(second_range.end());
    }

    if result < 0 {
        ComparePos::Before
    } else if result > 0 {
        ComparePos::After
    } else {
        ComparePos::Equal
    }
}

pub fn containing_file_for_node(node: SyntaxNode) -> Option<SourceFile> {
    let mut node = node;
    while node.kind() != SOURCE_FILE {
        node = node.parent()?;
    }
    SourceFile::cast(node)
}

pub fn containing_file_for_token(token: SyntaxToken) -> Option<SourceFile> {
    let mut node = token.parent()?;
    while node.kind() != SOURCE_FILE {
        node = node.parent()?;
    }
    SourceFile::cast(node)
}

/// Returns ancestors of the node at the offset, sorted by length. This should
/// do the right thing at an edge, e.g. when searching for expressions at `{
/// $0foo }` we will get the name reference instead of the whole block, which
/// we would get if we just did `find_token_at_offset(...).flat_map(|t|
/// t.parent().ancestors())`.
pub fn ancestors_at_offset(node: &SyntaxNode, offset: TextSize) -> impl Iterator<Item = SyntaxNode> {
    node.token_at_offset(offset)
        .map(|token| token.parent_ancestors())
        .kmerge_by(|node1, node2| node1.text_range().len() < node2.text_range().len())
}

/// Finds a node of specific Ast type at offset. Note that this is slightly
/// imprecise: if the cursor is strictly between two nodes of the desired type,
/// as in
///
/// ```no_run
/// struct Foo {}/*caret*/struct Bar;
/// ```
///
/// then the shorter node will be silently preferred.
pub fn find_node_at_offset<N: AstNode>(syntax: &SyntaxNode, offset: TextSize) -> Option<N> {
    ancestors_at_offset(syntax, offset).find_map(N::cast)
}

pub fn find_node_at_range<N: AstNode>(syntax: &SyntaxNode, range: TextRange) -> Option<N> {
    syntax.covering_element(range).ancestors().find_map(N::cast)
}

/// Skip to next non `trivia` token
pub fn skip_trivia_token(mut token: SyntaxToken, direction: Direction) -> Option<SyntaxToken> {
    while token.kind().is_trivia() {
        token = match direction {
            Direction::Next => token.next_token()?,
            Direction::Prev => token.prev_token()?,
        }
    }
    Some(token)
}

/// Skip to next non `whitespace` token
pub fn skip_whitespace_token(mut token: SyntaxToken, direction: Direction) -> Option<SyntaxToken> {
    while token.kind() == SyntaxKind::WHITESPACE {
        token = match direction {
            Direction::Next => token.next_token()?,
            Direction::Prev => token.prev_token()?,
        }
    }
    Some(token)
}

/// Finds the first sibling in the given direction which is not `trivia`
pub fn non_trivia_sibling(element: SyntaxElement, direction: Direction) -> Option<SyntaxElement> {
    return match element {
        NodeOrToken::Node(node) => node.siblings_with_tokens(direction).skip(1).find(not_trivia),
        NodeOrToken::Token(token) => token.siblings_with_tokens(direction).skip(1).find(not_trivia),
    };

    fn not_trivia(element: &SyntaxElement) -> bool {
        match element {
            NodeOrToken::Node(_) => true,
            NodeOrToken::Token(token) => !token.kind().is_trivia(),
        }
    }
}

pub fn least_common_ancestor(u: &SyntaxNode, v: &SyntaxNode) -> Option<SyntaxNode> {
    if u == v {
        return Some(u.clone());
    }

    let u_depth = u.ancestors().count();
    let v_depth = v.ancestors().count();
    let keep = u_depth.min(v_depth);

    let u_candidates = u.ancestors().skip(u_depth - keep);
    let v_candidates = v.ancestors().skip(v_depth - keep);
    let (res, _) = u_candidates.zip(v_candidates).find(|(x, y)| x == y)?;
    Some(res)
}

pub fn neighbor<T: AstNode>(me: &T, direction: Direction) -> Option<T> {
    me.syntax().siblings(direction).skip(1).find_map(T::cast)
}

pub fn has_errors(node: &SyntaxNode) -> bool {
    node.children().any(|it| it.kind() == SyntaxKind::ERROR)
}
