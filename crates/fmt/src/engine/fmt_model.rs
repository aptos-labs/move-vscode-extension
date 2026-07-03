use itertools::Itertools;
use std::fmt;
use std::ptr::NonNull;
use syntax::SyntaxKind::*;
use syntax::ast::{self};
use syntax::{AstNode, SyntaxElement, SyntaxKind, SyntaxToken, TextRange};

#[derive(Clone, Copy)]
pub(crate) enum Direction {
    LeftToRight,
    RightToLeft,
}

pub(crate) enum FmtLeaf<'a> {
    Token(&'a SyntaxToken),
    WS(&'a str),
}

impl FmtLeaf<'_> {
    pub(crate) fn text_len(&self) -> usize {
        match self {
            FmtLeaf::Token(token) => token.text().len(),
            FmtLeaf::WS(ws) => ws.len(),
        }
    }

    pub(crate) fn has_line_break(&self) -> bool {
        match self {
            FmtLeaf::Token(_) => false,
            FmtLeaf::WS(ws) => has_line_break(ws),
        }
    }
}

pub(crate) struct FmtBlockModel {
    root: Box<FmtBlock>,
}

/// Indent type relative to parent, applied when preceded by a line break.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IndentType {
    /// No additional indent.
    None,
    /// One block indent level (e.g. body of a function, struct fields).
    Block,
    /// Continuation indent (e.g. wrapped expression after `=`).
    Continuation,
}

pub(crate) struct FmtBlock {
    pub(crate) syntax_element: SyntaxElement,
    pub(crate) parent: Option<*const FmtBlock>,
    /// Indent relative to parent when preceded by a line break.
    pub(crate) indent_type: IndentType,
    /// Whitespace immediately preceding this block.
    pub(crate) ws_before: String,
    /// Empty for leaf tokens.
    pub(crate) children: Box<[FmtBlock]>,
}

type FmtBlockIndex = usize;

impl FmtBlock {
    pub(crate) fn new(
        syntax_element: SyntaxElement,
        indent_type: IndentType,
        ws_before: String,
        children: Vec<FmtBlock>,
    ) -> Self {
        FmtBlock {
            syntax_element,
            parent: None,
            indent_type,
            ws_before,
            children: children.into_boxed_slice(),
        }
    }

    pub(crate) fn indent_type(&self) -> IndentType {
        self.indent_type
    }

    pub(crate) fn syntax_element(&self) -> &SyntaxElement {
        &self.syntax_element
    }

    pub(crate) fn kind(&self) -> SyntaxKind {
        self.syntax_element.kind()
    }

    pub(crate) fn text_range(&self) -> TextRange {
        self.syntax_element.text_range()
    }

    pub(crate) fn is_token(&self) -> bool {
        self.children.is_empty()
    }

    pub(crate) fn parent(&self) -> Option<&FmtBlock> {
        // SAFETY: parent pointers are bound after all boxed child slices reach
        // their final allocations and the tree structure is immutable afterward.
        self.parent.map(|p| unsafe { &*p })
    }

    pub(crate) fn index_in_parent(&self) -> Option<usize> {
        let parent = self.parent()?;
        let self_ptr = self as *const FmtBlock;
        parent
            .children
            .iter()
            .position(|child| std::ptr::eq(self_ptr, child as *const FmtBlock))
    }

    pub(crate) fn ws_before(&self) -> &str {
        &self.ws_before
    }

    pub(crate) fn ws_before_mut(&mut self) -> &mut String {
        &mut self.ws_before
    }

    pub(crate) fn ws_has_line_break(&self) -> bool {
        self.ws_before.contains('\n')
    }

    pub(crate) fn set_ws(&mut self, ws: &str) {
        ws.clone_into(&mut self.ws_before);
    }

    pub(crate) fn left_sibling(&self) -> Option<&FmtBlock> {
        let parent = self.parent()?;
        let self_idx = self.index_in_parent()?;
        self_idx.checked_sub(1).and_then(|idx| parent.children.get(idx))
    }

    pub(crate) fn right_sibling(&self) -> Option<&FmtBlock> {
        let parent = self.parent()?;
        let self_idx = self.index_in_parent()?;
        parent.children.get(self_idx + 1)
    }

    pub(crate) fn children_ws(&self) -> impl Iterator<Item = &str> {
        self.children.iter().map(|child| child.ws_before())
    }

    pub(crate) fn child_blocks(&self) -> impl Iterator<Item = &FmtBlock> {
        self.children.iter()
    }

    pub(crate) fn child_blocks_mut(&mut self) -> impl Iterator<Item = &mut FmtBlock> {
        self.children.iter_mut()
    }

    pub(crate) fn for_each_descendant_mut(
        &mut self,
        kind: SyntaxKind,
        f: &mut impl FnMut(&mut FmtBlock),
    ) {
        if self.kind() == kind {
            f(self);
        }

        for child in self.child_blocks_mut() {
            child.for_each_descendant_mut(kind, f);
        }
    }

    pub(crate) fn for_each_descendant_pattern_mut(
        &mut self,
        parent_kind: Option<SyntaxKind>,
        left_kind: SyntaxKind,
        right_kind: SyntaxKind,
        f: &mut impl FnMut(&mut FmtBlock, FmtBlockIndex, FmtBlockIndex),
    ) {
        if parent_kind.is_none_or(|parent_kind| self.kind() == parent_kind) {
            let index_pairs = (0..self.children.len())
                .tuple_windows::<(usize, usize)>()
                .collect::<Vec<_>>();
            for (left, right) in index_pairs {
                if self.child_block_kind(left) == Some(left_kind)
                    && self.child_block_kind(right) == Some(right_kind)
                {
                    f(self, left, right);
                }
            }
        }

        for child in self.child_blocks_mut() {
            child.for_each_descendant_pattern_mut(parent_kind, left_kind, right_kind, f);
        }
    }

    pub(crate) fn for_each_descendant_block_mut(&mut self, f: &mut impl FnMut(&mut FmtBlock)) {
        f(self);

        for child in self.child_blocks_mut() {
            child.for_each_descendant_block_mut(f);
        }
    }

    pub(crate) fn for_each_descendant_ws_mut(&mut self, f: &mut impl FnMut(&mut String)) {
        for child in self.children_mut() {
            f(child.ws_before_mut());
            child.for_each_descendant_ws_mut(f);
        }
    }

    /// Remove spaces from the indentation after line breaks in every
    /// descendant whitespace entry that contains a line break.
    /// For example, if the block contains `"\n\n        "`, `dedent_by(4)`
    /// changes that whitespace to `"\n\n    "`.
    pub(crate) fn dedent_by(&mut self, dedent_size: usize) {
        self.for_each_descendant_ws_mut(&mut |ws| {
            if has_line_break(ws) {
                dedent_ws_by(ws, dedent_size);
            }
        });
    }

    /// Add spaces to the indentation after the last line break in every
    /// descendant whitespace entry that contains a line break.
    /// For example, if the block contains `"\n\n    "`, `indent_by(4)`
    /// changes that whitespace to `"\n\n        "`.
    pub(crate) fn indent_by(&mut self, indent_size: usize) {
        self.for_each_descendant_ws_mut(&mut |ws| {
            if has_line_break(ws) {
                indent_ws_by(ws, indent_size);
            }
        });
    }

    /// Measure visible line length at `children[child_idx].ws_before`, walking
    /// outward in `direction` and stopping at the first line break.
    ///
    /// The selected `ws_before` is excluded, so callers can compute
    /// `left + replacement_len + right` even when it is multiline.
    pub(crate) fn line_len_at(&self, child_idx: usize, direction: Direction) -> usize {
        let mut len = 0;
        let mut pending_ws_len = 0;
        for leaf in leafs_from_excluding_self(self, child_idx, direction) {
            match (direction, leaf) {
                // stop counting if we're at the comment
                (_, FmtLeaf::Token(token)) if token.kind() == COMMENT => {
                    // note that we're ignoring `pending_ws_len` for this token
                    break;
                }
                // A multiline whitespace bounds the current line. Include only
                // the part of that whitespace that belongs to this line, then stop:
                // walking right through `"abc\n    "` contributes `3`;
                // walking left through `"\n        "` contributes indent `8`.
                (_, FmtLeaf::WS(ws)) if has_line_break(ws) => {
                    len += match direction {
                        Direction::LeftToRight => ws.find('\n').unwrap(),
                        Direction::RightToLeft => ws_indent(ws),
                    };
                    break;
                }
                (Direction::LeftToRight, FmtLeaf::WS(ws)) => {
                    pending_ws_len += ws.len();
                }
                (Direction::LeftToRight, FmtLeaf::Token(token)) => {
                    len += pending_ws_len + token.text().len();
                    pending_ws_len = 0;
                }
                (Direction::RightToLeft, leaf) => len += leaf.text_len(),
            }
        }
        len
    }

    /// Iterate over all leaf entries (tokens and WS) in the subtree.
    pub(crate) fn leafs(&self, direction: Direction) -> FmtLeafIter<'_> {
        let mut stack = Vec::new();
        if !self.is_token() {
            push_children(&mut stack, &self.children, direction);
        }
        FmtLeafIter { stack, direction }
    }

    pub(crate) fn children(&self) -> &[FmtBlock] {
        &self.children
    }

    pub(crate) fn child_block_kind(&self, block_idx: usize) -> Option<SyntaxKind> {
        self.children().get(block_idx).map(FmtBlock::kind)
    }

    pub(crate) fn child_block(&self, i: usize) -> Option<&FmtBlock> {
        self.children().get(i)
    }

    pub(crate) fn child_block_mut(&mut self, i: usize) -> Option<&mut FmtBlock> {
        self.children_mut().get_mut(i)
    }

    pub(crate) fn children_mut(&mut self) -> &mut [FmtBlock] {
        &mut self.children
    }

    fn write_text(&self, out: &mut String) {
        out.push_str(&self.ws_before);
        if self.is_token() {
            out.push_str(self.syntax_element.as_token().unwrap().text());
            return;
        }
        for child in &self.children {
            child.write_text(out);
        }
    }
}

impl fmt::Debug for FmtBlockModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.root.write_debug(f, 0)
    }
}

impl fmt::Debug for FmtBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_debug(f, 0)
    }
}

impl FmtBlock {
    fn write_debug(&self, f: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
        if self.is_token() {
            return self.write_token_debug(f, depth);
        }

        let range = self.text_range();
        write_indent(f, depth)?;
        writeln!(
            f,
            "<{:?} {}:{} <Indent: {}>>",
            self.kind(),
            u32::from(range.start()),
            u32::from(range.end()),
            DebugIndent(self.indent_type),
        )?;

        for (idx, child) in self.children.iter().enumerate() {
            if idx != 0 || !child.ws_before.is_empty() {
                write_ws_debug(&child.ws_before, f, depth + 1)?;
            }
            child.write_debug(f, depth + 1)?;
        }

        Ok(())
    }

    fn write_token_debug(&self, f: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
        let token = self
            .syntax_element()
            .as_token()
            .expect("token block has token syntax element");
        let range = token.text_range();
        write_indent(f, depth)?;
        writeln!(
            f,
            "{:?} {}:{} <Indent: {}>",
            token.text(),
            u32::from(range.start()),
            u32::from(range.end()),
            DebugIndent(self.indent_type),
        )
    }
}

fn write_ws_debug(ws: &str, f: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
    write_indent(f, depth)?;
    writeln!(f, "ws: {ws:?}")
}

struct DebugIndent(IndentType);

impl fmt::Display for DebugIndent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            IndentType::None => f.write_str("NONE"),
            IndentType::Block => f.write_str("BLOCK"),
            IndentType::Continuation => f.write_str("CONTINUATION"),
        }
    }
}

fn write_indent(f: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
    for _ in 0..depth {
        f.write_str("  ")?;
    }
    Ok(())
}

pub(crate) struct FmtLeafIter<'a> {
    stack: Vec<FmtLeafSource<'a>>,
    direction: Direction,
}

enum FmtLeafSource<'a> {
    Block(&'a FmtBlock),
    WS(&'a str),
}

impl<'a> Iterator for FmtLeafIter<'a> {
    type Item = FmtLeaf<'a>;

    fn next(&mut self) -> Option<FmtLeaf<'a>> {
        while let Some(source) = self.stack.pop() {
            match source {
                FmtLeafSource::WS(ws) => return Some(FmtLeaf::WS(ws)),
                FmtLeafSource::Block(block) => {
                    if block.is_token() {
                        return Some(FmtLeaf::Token(block.syntax_element.as_token().unwrap()));
                    }
                    push_children(&mut self.stack, &block.children, self.direction);
                }
            }
        }
        None
    }
}

/// Iterate leaves starting from `parent.children[idx]` in the given direction,
/// crossing into child blocks and continuing up through parents when siblings
/// are exhausted.
pub(crate) fn leafs_from_excluding_self(
    parent: &FmtBlock,
    child_idx: usize,
    direction: Direction,
) -> LeavesFromIter<'_> {
    let mut stack = Vec::new();
    match direction {
        Direction::LeftToRight => {
            push_children(&mut stack, &parent.children[child_idx + 1..], direction);
            stack.push(FmtLeafSource::Block(&parent.children[child_idx]));
        }
        Direction::RightToLeft => {
            push_children(&mut stack, &parent.children[..child_idx], direction);
        }
    }
    LeavesFromIter {
        inner: FmtLeafIter { stack, direction },
        current_block: Some(parent),
        direction,
    }
}

pub(crate) struct LeavesFromIter<'a> {
    inner: FmtLeafIter<'a>,
    /// The block whose children we're currently iterating.
    /// When inner is exhausted, we go up to this block's parent.
    current_block: Option<&'a FmtBlock>,
    direction: Direction,
}

impl<'a> Iterator for LeavesFromIter<'a> {
    type Item = FmtLeaf<'a>;

    fn next(&mut self) -> Option<FmtLeaf<'a>> {
        loop {
            if let Some(leaf) = self.inner.next() {
                return Some(leaf);
            }
            // Inner iterator exhausted — go up to parent.
            let block = self.current_block?;
            let parent = block.parent()?;

            let block_idx = block.index_in_parent()?;
            match self.direction {
                Direction::LeftToRight => {
                    push_children(
                        &mut self.inner.stack,
                        &parent.children[block_idx + 1..],
                        self.direction,
                    );
                }
                Direction::RightToLeft => {
                    push_children(
                        &mut self.inner.stack,
                        &parent.children[..block_idx],
                        self.direction,
                    );
                    self.inner.stack.push(FmtLeafSource::WS(block.ws_before()));
                }
            }
            self.current_block = Some(parent);
        }
    }
}

fn push_children<'a>(
    stack: &mut Vec<FmtLeafSource<'a>>,
    children: &'a [FmtBlock],
    direction: Direction,
) {
    match direction {
        Direction::LeftToRight => {
            for child in children.iter().rev() {
                stack.push(FmtLeafSource::Block(child));
                stack.push(FmtLeafSource::WS(child.ws_before()));
            }
        }
        Direction::RightToLeft => {
            for child in children {
                stack.push(FmtLeafSource::WS(child.ws_before()));
                stack.push(FmtLeafSource::Block(child));
            }
        }
    }
}

impl FmtBlockModel {
    pub(crate) fn new(file: &ast::SourceFile) -> FmtBlockModel {
        let mut root = Box::new(FmtBlock::build(
            file.syntax().clone().into(),
            String::new(),
            IndentType::None,
        ));
        bind_parent_links(&mut root, None);
        FmtBlockModel { root }
    }

    pub(crate) fn into_text(self) -> String {
        let mut out = String::new();
        self.root.write_text(&mut out);
        out
    }

    pub(crate) fn root(&self) -> &FmtBlock {
        &self.root
    }

    pub(crate) fn root_mut(&mut self) -> &mut FmtBlock {
        &mut self.root
    }

    pub(crate) fn for_each_descendant_mut(
        &mut self,
        kind: SyntaxKind,
        mut f: impl FnMut(&mut FmtBlock),
    ) {
        self.root.for_each_descendant_mut(kind, &mut f);
    }

    pub(crate) fn for_each_descendant_pattern_mut(
        &mut self,
        parent_kind: Option<SyntaxKind>,
        left_kind: SyntaxKind,
        right_kind: SyntaxKind,
        mut f: impl FnMut(&mut FmtBlock, FmtBlockIndex, FmtBlockIndex),
    ) {
        self.root
            .for_each_descendant_pattern_mut(parent_kind, left_kind, right_kind, &mut f);
    }

    pub(crate) fn for_each_block_mut(&mut self, mut f: impl FnMut(&mut FmtBlock)) {
        self.root.for_each_descendant_block_mut(&mut f);
    }
}

fn bind_parent_links(block: &mut FmtBlock, parent_ptr: Option<*const FmtBlock>) {
    block.parent = parent_ptr;
    let block_ptr = block as *const FmtBlock;
    for child in block.children.iter_mut() {
        bind_parent_links(child, Some(block_ptr));
    }
}

pub(crate) fn has_line_break(ws: &str) -> bool {
    ws.contains('\n')
}

pub(crate) fn ws_indent(ws: &str) -> usize {
    match ws.rfind('\n') {
        Some(pos) => ws.len() - pos - 1,
        None => 0,
    }
}

pub(crate) fn dedent_ws_by(ws: &mut String, dedent_size: usize) {
    *ws = stdx::dedent_by(dedent_size, ws);
}

pub(crate) fn indent_ws_by(ws: &mut String, indent_size: usize) {
    let Some(last_newline_idx) = ws.rfind('\n') else {
        return;
    };
    ws.insert_str(last_newline_idx + 1, &" ".repeat(indent_size));
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::{Expect, expect};

    fn check_fmt_model_debug(source: &str, expect: Expect) {
        let parse = ast::SourceFile::parse(source);
        let file = parse.tree();
        let model = FmtBlockModel::new(&file);
        expect.assert_eq(&format!("{model:#?}"));
    }

    #[test]
    fn test_roundtrip_simple() {
        let source = "module 0x1::m { fun main() { let x = 1; } }";
        let parse = ast::SourceFile::parse(source);
        let file = parse.tree();
        let model = FmtBlockModel::new(&file);
        assert_eq!(model.into_text(), source);
    }

    #[test]
    fn test_roundtrip_with_whitespace() {
        // Trailing \n is dropped — handled by format_content, not block model.
        let source = "module 0x1::m {\n    fun main() {\n        let x = 1;\n    }\n}\n";
        let expected = "module 0x1::m {\n    fun main() {\n        let x = 1;\n    }\n}";
        let parse = ast::SourceFile::parse(source);
        let file = parse.tree();
        let model = FmtBlockModel::new(&file);
        assert_eq!(model.into_text(), expected);
    }

    #[test]
    fn test_debug_fmt_model_prints_blocks_tokens_and_spacing() {
        check_fmt_model_debug(
            "module 0x1::m {}",
            expect![[r#"
                <SOURCE_FILE 0:16 <Indent: NONE>>
                  <MODULE 0:16 <Indent: NONE>>
                    "module" 0:6 <Indent: NONE>
                    ws: " "
                    <VALUE_ADDRESS 7:10 <Indent: NONE>>
                      "0x1" 7:10 <Indent: NONE>
                    ws: ""
                    "::" 10:12 <Indent: NONE>
                    ws: ""
                    <NAME 12:13 <Indent: NONE>>
                      "m" 12:13 <Indent: NONE>
                    ws: " "
                    "{" 14:15 <Indent: NONE>
                    ws: ""
                    "}" 15:16 <Indent: NONE>
            "#]],
        );
    }

    #[test]
    fn test_debug_fmt_model_prints_multiline_spacing() {
        check_fmt_model_debug(
            // language=Move
            r#"
            module 0x1::m {
                fun main() {
                    let a = 1 + 1 + 1;
                }
            }
            "#,
            expect![[r#"
                <SOURCE_FILE 0:141 <Indent: NONE>>
                  ws: "\n            "
                  <MODULE 13:128 <Indent: NONE>>
                    "module" 13:19 <Indent: NONE>
                    ws: " "
                    <VALUE_ADDRESS 20:23 <Indent: NONE>>
                      "0x1" 20:23 <Indent: NONE>
                    ws: ""
                    "::" 23:25 <Indent: NONE>
                    ws: ""
                    <NAME 25:26 <Indent: NONE>>
                      "m" 25:26 <Indent: NONE>
                    ws: " "
                    "{" 27:28 <Indent: NONE>
                    ws: "\n                "
                    <FUN 45:114 <Indent: BLOCK>>
                      "fun" 45:48 <Indent: NONE>
                      ws: " "
                      <NAME 49:53 <Indent: NONE>>
                        "main" 49:53 <Indent: NONE>
                      ws: ""
                      <PARAM_LIST 53:55 <Indent: NONE>>
                        "(" 53:54 <Indent: NONE>
                        ws: ""
                        ")" 54:55 <Indent: NONE>
                      ws: " "
                      <BLOCK_EXPR 56:114 <Indent: NONE>>
                        "{" 56:57 <Indent: NONE>
                        ws: "\n                    "
                        <LET_STMT 78:96 <Indent: BLOCK>>
                          "let" 78:81 <Indent: NONE>
                          ws: " "
                          <IDENT_PAT 82:83 <Indent: NONE>>
                            <NAME 82:83 <Indent: NONE>>
                              "a" 82:83 <Indent: NONE>
                          ws: " "
                          "=" 84:85 <Indent: NONE>
                          ws: " "
                          <BIN_EXPR 86:95 <Indent: CONTINUATION>>
                            <LITERAL 86:87 <Indent: CONTINUATION>>
                              "1" 86:87 <Indent: NONE>
                            ws: " "
                            "+" 88:89 <Indent: CONTINUATION>
                            ws: " "
                            <LITERAL 90:91 <Indent: CONTINUATION>>
                              "1" 90:91 <Indent: NONE>
                            ws: " "
                            "+" 92:93 <Indent: CONTINUATION>
                            ws: " "
                            <LITERAL 94:95 <Indent: CONTINUATION>>
                              "1" 94:95 <Indent: NONE>
                          ws: ""
                          ";" 95:96 <Indent: NONE>
                        ws: "\n                "
                        "}" 113:114 <Indent: NONE>
                    ws: "\n            "
                    "}" 127:128 <Indent: NONE>
            "#]],
        );
    }

    #[test]
    fn test_roundtrip_no_whitespace() {
        let source = "module 0x1::m{}";
        let parse = ast::SourceFile::parse(source);
        let file = parse.tree();
        let model = FmtBlockModel::new(&file);
        assert_eq!(model.into_text(), source);
    }

    #[test]
    fn test_roundtrip_multiple_items() {
        let source = "module 0x1::m {\n    fun a() {}\n\n    fun b() {}\n}\n";
        let expected = "module 0x1::m {\n    fun a() {}\n\n    fun b() {}\n}";
        let parse = ast::SourceFile::parse(source);
        let file = parse.tree();
        let model = FmtBlockModel::new(&file);
        assert_eq!(model.into_text(), expected);
    }

    #[test]
    fn test_leaves_ltr() {
        let source = "module 0x1::m { fun main() {} }";
        let parse = ast::SourceFile::parse(source);
        let file = parse.tree();
        let model = FmtBlockModel::new(&file);

        let text: String = model
            .root()
            .leafs(Direction::LeftToRight)
            .map(|leaf| match leaf {
                FmtLeaf::Token(token) => token.text().to_string(),
                FmtLeaf::WS(ws) => ws.to_string(),
            })
            .collect();
        assert_eq!(text, source);
    }

    #[test]
    fn test_leaves_rtl() {
        let source = "module 0x1::m { fun main() {} }";
        let parse = ast::SourceFile::parse(source);
        let file = parse.tree();
        let model = FmtBlockModel::new(&file);

        let text: String = model
            .root()
            .leafs(Direction::RightToLeft)
            .map(|leaf| match leaf {
                FmtLeaf::Token(token) => token.text().to_string(),
                FmtLeaf::WS(ws) => ws.to_string(),
            })
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        assert_eq!(text, source);
    }

    #[test]
    fn test_leaves_from_ltr() {
        // "module 0x1::m { fun main() {} }"
        // Find the WS between "{" and "fun", then iterate right from there.
        let source = "module 0x1::m { fun main() {} }";
        let parse = ast::SourceFile::parse(source);
        let file = parse.tree();
        let model = FmtBlockModel::new(&file);

        // root > MODULE > children: ["module", PATH, "{", FUN, "}"]
        let module = model.root().child_blocks().next().unwrap();
        let fun_block = module.child_blocks().find(|b| b.kind() == FUN).unwrap();
        let fun_idx = fun_block.index_in_parent().unwrap();

        let right_text: String = leafs_from_excluding_self(module, fun_idx, Direction::LeftToRight)
            .map(|leaf| match leaf {
                FmtLeaf::Token(token) => token.text().to_string(),
                FmtLeaf::WS(ws) => ws.to_string(),
            })
            .collect();
        // Everything from after the WS before "fun" to the end: "fun main() {} }"
        // (includes the WS and "}" from the MODULE level too)
        assert_eq!(right_text, "fun main() {} }");
    }

    #[test]
    fn test_leaves_from_rtl() {
        let source = "module 0x1::m { fun main() {} }";
        let parse = ast::SourceFile::parse(source);
        let file = parse.tree();
        let model = FmtBlockModel::new(&file);

        let module = model.root().child_blocks().next().unwrap();
        let fun_block = module.child_blocks().find(|b| b.kind() == FUN).unwrap();
        let fun_idx = fun_block.index_in_parent().unwrap();

        let left_text: String = leafs_from_excluding_self(module, fun_idx, Direction::RightToLeft)
            .map(|leaf| match leaf {
                FmtLeaf::Token(token) => token.text().to_string(),
                FmtLeaf::WS(ws) => ws.to_string(),
            })
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        // Everything from the start to before the WS: "module 0x1::m {"
        assert_eq!(left_text, "module 0x1::m {");
    }

    #[test]
    fn test_parent_pointer() {
        let source = "module 0x1::m { fun main() {} }";
        let parse = ast::SourceFile::parse(source);
        let file = parse.tree();
        let model = FmtBlockModel::new(&file);

        // root has no parent
        assert!(model.root().parent().is_none());

        // first block child of root should have root as parent
        let root = model.root();
        let first_child = root.child_blocks().next().unwrap();
        let parent = first_child.parent().unwrap();
        assert_eq!(parent.kind(), root.kind());
    }
}
