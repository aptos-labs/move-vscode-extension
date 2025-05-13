use ide_db::{RootDatabase, root_db};
use lang::Semantics;
use line_index::{LineCol, LineIndex};
use std::sync::Arc;
use stdx::format_to;
use syntax::{AstNode, NodeOrToken, SyntaxNode, TextSize, WalkEvent};
use vfs::FileId;

// Feature: Show Syntax Tree
//
// Shows a tree view with the syntax tree of the current file
//
// | Editor  | Panel Name |
// |---------|-------------|
// | VS Code | **Rust Syntax Tree** |
pub(crate) fn view_syntax_tree(db: &RootDatabase, file_id: FileId) -> String {
    let sema = Semantics::new(db, file_id);
    let line_index = root_db::line_index(db, file_id);
    let parse = sema.parse(file_id);

    let ctx = SyntaxTreeCtx { line_index };

    syntax_node_to_json(parse.syntax(), &ctx)
}

fn syntax_node_to_json(root_node: &SyntaxNode, ctx: &SyntaxTreeCtx) -> String {
    let mut result = String::new();
    for event in root_node.preorder_with_tokens() {
        match event {
            WalkEvent::Enter(elem) => {
                let kind = elem.kind();
                let start = TextPosition::new(&ctx.line_index, elem.text_range().start());
                let end = TextPosition::new(&ctx.line_index, elem.text_range().end());

                match elem {
                    NodeOrToken::Node(_) => {
                        format_to!(
                            result,
                            r#"{{"type":"Node","kind":"{kind:?}","start":{start},"end":{end},"children":["#
                        );
                    }
                    NodeOrToken::Token(token) => {
                        let comma = if token.next_sibling_or_token().is_some() {
                            ","
                        } else {
                            ""
                        };
                        format_to!(
                            result,
                            r#"{{"type":"Token","kind":"{kind:?}","start":{start},"end":{end}}}{comma}"#
                        );
                    }
                }
            }
            WalkEvent::Leave(elem) => match elem {
                NodeOrToken::Node(node) => {
                    let comma = if node.next_sibling_or_token().is_some() {
                        ","
                    } else {
                        ""
                    };
                    format_to!(result, "]}}{comma}")
                }
                NodeOrToken::Token(_) => (),
            },
        }
    }

    result
}

struct TextPosition {
    offset: TextSize,
    line: u32,
    col: u32,
}

impl std::fmt::Display for TextPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?},{},{}]", self.offset, self.line, self.col)
    }
}

impl TextPosition {
    pub(crate) fn new(line_index: &LineIndex, offset: TextSize) -> Self {
        let LineCol { line, col } = line_index.line_col(offset);
        Self { offset, line, col }
    }
}

struct SyntaxTreeCtx {
    line_index: Arc<LineIndex>,
}
