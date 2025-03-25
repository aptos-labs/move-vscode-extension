use ide_db::RootDatabase;
use lang::Semantics;
use syntax::{
    Direction, NodeOrToken,
    SyntaxKind::{self, *},
    SyntaxNode, SyntaxToken, T, TextRange, TextSize, TokenAtOffset,
    ast::{self, AstNode, AstToken},
};

use crate::FileRange;

// Feature: Expand and Shrink Selection
//
// Extends or shrinks the current selection to the encompassing syntactic construct
// (expression, statement, item, module, etc). It works with multiple cursors.
//
// | Editor  | Shortcut |
// |---------|----------|
// | VS Code | <kbd>Alt+Shift+→</kbd>, <kbd>Alt+Shift+←</kbd> |
//
// ![Expand and Shrink Selection](https://user-images.githubusercontent.com/48062697/113020651-b42fc800-917a-11eb-8a4f-cf1a07859fac.gif)
pub(crate) fn extend_selection(db: &RootDatabase, frange: FileRange) -> TextRange {
    let sema = Semantics::new(db);
    let file = sema.parse(frange.file_id);
    try_extend_selection(file.syntax(), frange).unwrap_or(frange.range)
}

fn try_extend_selection(root: &SyntaxNode, frange: FileRange) -> Option<TextRange> {
    let range = frange.range;

    let string_kinds = [COMMENT, BYTE_STRING];
    let list_kinds = [
        STRUCT_PAT_FIELD_LIST,
        MATCH_ARM_LIST,
        NAMED_FIELD_LIST,
        TUPLE_FIELD_LIST,
        // RECORD_EXPR_FIELD_LIST,
        VARIANT_LIST,
        // USE_GROUP,
        TYPE_PARAM_LIST,
        TYPE_ARG_LIST,
        // TYPE_BOUND_LIST,
        PARAM_LIST,
        ARG_LIST,
        VECTOR_LIT_EXPR,
        TUPLE_EXPR,
        TUPLE_TYPE,
        TUPLE_PAT,
        // WHERE_CLAUSE,
    ];

    if range.is_empty() {
        let offset = range.start();
        let mut leaves = root.token_at_offset(offset);
        if leaves.clone().all(|it| it.kind() == WHITESPACE) {
            return Some(extend_ws(root, leaves.next()?, offset));
        }
        let leaf_range = match leaves {
            TokenAtOffset::None => return None,
            TokenAtOffset::Single(l) => {
                if string_kinds.contains(&l.kind()) {
                    extend_single_word_in_comment_or_string(&l, offset).unwrap_or_else(|| l.text_range())
                } else {
                    l.text_range()
                }
            }
            TokenAtOffset::Between(l, r) => pick_best(l, r).text_range(),
        };
        return Some(leaf_range);
    };
    let node = match root.covering_element(range) {
        NodeOrToken::Token(token) => {
            if token.text_range() != range {
                return Some(token.text_range());
            }
            if let Some(comment) = ast::Comment::cast(token.clone()) {
                if let Some(range) = extend_comments(comment) {
                    return Some(range);
                }
            }
            token.parent()?
        }
        NodeOrToken::Node(node) => node,
    };

    if node.text_range() != range {
        return Some(node.text_range());
    }

    let node = shallowest_node(&node);

    if node.parent().is_some_and(|n| list_kinds.contains(&n.kind())) {
        if let Some(range) = extend_list_item(&node) {
            return Some(range);
        }
    }

    node.parent().map(|it| it.text_range())
}

/// Find the shallowest node with same range, which allows us to traverse siblings.
fn shallowest_node(node: &SyntaxNode) -> SyntaxNode {
    node.ancestors()
        .take_while(|n| n.text_range() == node.text_range())
        .last()
        .unwrap()
}

fn extend_single_word_in_comment_or_string(leaf: &SyntaxToken, offset: TextSize) -> Option<TextRange> {
    let text: &str = leaf.text();
    let cursor_position: u32 = (offset - leaf.text_range().start()).into();

    let (before, after) = text.split_at(cursor_position as usize);

    fn non_word_char(c: char) -> bool {
        !(c.is_alphanumeric() || c == '_')
    }

    let start_idx = before.rfind(non_word_char)? as u32;
    let end_idx = after.find(non_word_char).unwrap_or(after.len()) as u32;

    // FIXME: use `ceil_char_boundary` from `std::str` when it gets stable
    // https://github.com/rust-lang/rust/issues/93743
    fn ceil_char_boundary(text: &str, index: u32) -> u32 {
        (index..)
            .find(|&index| text.is_char_boundary(index as usize))
            .unwrap_or(text.len() as u32)
    }

    let from: TextSize = ceil_char_boundary(text, start_idx + 1).into();
    let to: TextSize = (cursor_position + end_idx).into();

    let range = TextRange::new(from, to);
    if range.is_empty() {
        None
    } else {
        Some(range + leaf.text_range().start())
    }
}

fn extend_ws(root: &SyntaxNode, ws: SyntaxToken, offset: TextSize) -> TextRange {
    let ws_text = ws.text();
    let suffix = TextRange::new(offset, ws.text_range().end()) - ws.text_range().start();
    let prefix = TextRange::new(ws.text_range().start(), offset) - ws.text_range().start();
    let ws_suffix = &ws_text[suffix];
    let ws_prefix = &ws_text[prefix];
    if ws_text.contains('\n') && !ws_suffix.contains('\n') {
        if let Some(node) = ws.next_sibling_or_token() {
            let start = match ws_prefix.rfind('\n') {
                Some(idx) => ws.text_range().start() + TextSize::from((idx + 1) as u32),
                None => node.text_range().start(),
            };
            let end = if root.text().char_at(node.text_range().end()) == Some('\n') {
                node.text_range().end() + TextSize::of('\n')
            } else {
                node.text_range().end()
            };
            return TextRange::new(start, end);
        }
    }
    ws.text_range()
}

fn pick_best(l: SyntaxToken, r: SyntaxToken) -> SyntaxToken {
    return if priority(&r) > priority(&l) { r } else { l };
    fn priority(n: &SyntaxToken) -> usize {
        match n.kind() {
            WHITESPACE => 0,
            IDENT => 2,
            _ => 1,
        }
    }
}

/// Extend list item selection to include nearby delimiter and whitespace.
fn extend_list_item(node: &SyntaxNode) -> Option<TextRange> {
    fn is_single_line_ws(node: &SyntaxToken) -> bool {
        node.kind() == WHITESPACE && !node.text().contains('\n')
    }

    fn nearby_delimiter(
        delimiter_kind: SyntaxKind,
        node: &SyntaxNode,
        dir: Direction,
    ) -> Option<SyntaxToken> {
        node.siblings_with_tokens(dir)
            .skip(1)
            .find(|node| match node {
                NodeOrToken::Node(_) => true,
                NodeOrToken::Token(it) => !is_single_line_ws(it),
            })
            .and_then(|it| it.into_token())
            .filter(|node| node.kind() == delimiter_kind)
    }

    let delimiter = match node.kind() {
        // TYPE_BOUND => T![+],
        _ => T![,],
    };

    if let Some(delimiter_node) = nearby_delimiter(delimiter, node, Direction::Next) {
        // Include any following whitespace when delimiter is after list item.
        let final_node = delimiter_node
            .next_sibling_or_token()
            .and_then(|it| it.into_token())
            .filter(is_single_line_ws)
            .unwrap_or(delimiter_node);

        return Some(TextRange::new(
            node.text_range().start(),
            final_node.text_range().end(),
        ));
    }
    if let Some(delimiter_node) = nearby_delimiter(delimiter, node, Direction::Prev) {
        return Some(TextRange::new(
            delimiter_node.text_range().start(),
            node.text_range().end(),
        ));
    }

    None
}

fn extend_comments(comment: ast::Comment) -> Option<TextRange> {
    let prev = adj_comments(&comment, Direction::Prev);
    let next = adj_comments(&comment, Direction::Next);
    if prev != next {
        Some(TextRange::new(
            prev.syntax().text_range().start(),
            next.syntax().text_range().end(),
        ))
    } else {
        None
    }
}

fn adj_comments(comment: &ast::Comment, dir: Direction) -> ast::Comment {
    let mut res = comment.clone();
    for element in comment.syntax().siblings_with_tokens(dir) {
        let token = match element.as_token() {
            None => break,
            Some(token) => token,
        };
        if let Some(c) = ast::Comment::cast(token.clone()) {
            res = c
        } else if token.kind() != WHITESPACE || token.text().contains("\n\n") {
            break;
        }
    }
    res
}
