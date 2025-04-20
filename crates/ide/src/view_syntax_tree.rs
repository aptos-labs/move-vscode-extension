use ide_db::{LineIndexDatabase, RootDatabase};
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
    let line_index = db.line_index(file_id);
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

#[cfg(test)]
mod tests {
    use crate::Analysis;
    use expect_test::expect;

    fn check(source: &str, expect: expect_test::Expect) {
        let (analysis, file_id) = Analysis::from_single_file(source.to_string());
        let syn = analysis.view_syntax_tree(file_id).unwrap();
        expect.assert_eq(&syn);
    }

    #[test]
    fn test_view_syntax_tree() {
        check(
            r#"module 0x1::m { fun main() { call(1, 1); } }"#,
            expect![[
                r#"{"type":"Node","kind":"SOURCE_FILE","start":[0,0,0],"end":[44,0,44],"children":[{"type":"Node","kind":"MODULE","start":[0,0,0],"end":[44,0,44],"children":[{"type":"Token","kind":"MODULE_KW","start":[0,0,0],"end":[6,0,6]},{"type":"Token","kind":"WHITESPACE","start":[6,0,6],"end":[7,0,7]},{"type":"Node","kind":"VALUE_ADDRESS","start":[7,0,7],"end":[10,0,10],"children":[{"type":"Token","kind":"INT_NUMBER","start":[7,0,7],"end":[10,0,10]}]},{"type":"Token","kind":"COLON_COLON","start":[10,0,10],"end":[12,0,12]},{"type":"Node","kind":"NAME","start":[12,0,12],"end":[13,0,13],"children":[{"type":"Token","kind":"IDENT","start":[12,0,12],"end":[13,0,13]}]},{"type":"Token","kind":"WHITESPACE","start":[13,0,13],"end":[14,0,14]},{"type":"Token","kind":"L_CURLY","start":[14,0,14],"end":[15,0,15]},{"type":"Token","kind":"WHITESPACE","start":[15,0,15],"end":[16,0,16]},{"type":"Node","kind":"FUN","start":[16,0,16],"end":[42,0,42],"children":[{"type":"Token","kind":"FUN_KW","start":[16,0,16],"end":[19,0,19]},{"type":"Token","kind":"WHITESPACE","start":[19,0,19],"end":[20,0,20]},{"type":"Node","kind":"NAME","start":[20,0,20],"end":[24,0,24],"children":[{"type":"Token","kind":"IDENT","start":[20,0,20],"end":[24,0,24]}]},{"type":"Node","kind":"PARAM_LIST","start":[24,0,24],"end":[26,0,26],"children":[{"type":"Token","kind":"L_PAREN","start":[24,0,24],"end":[25,0,25]},{"type":"Token","kind":"R_PAREN","start":[25,0,25],"end":[26,0,26]}]},{"type":"Token","kind":"WHITESPACE","start":[26,0,26],"end":[27,0,27]},{"type":"Node","kind":"BLOCK_EXPR","start":[27,0,27],"end":[42,0,42],"children":[{"type":"Token","kind":"L_CURLY","start":[27,0,27],"end":[28,0,28]},{"type":"Token","kind":"WHITESPACE","start":[28,0,28],"end":[29,0,29]},{"type":"Node","kind":"EXPR_STMT","start":[29,0,29],"end":[40,0,40],"children":[{"type":"Node","kind":"CALL_EXPR","start":[29,0,29],"end":[39,0,39],"children":[{"type":"Node","kind":"PATH","start":[29,0,29],"end":[33,0,33],"children":[{"type":"Node","kind":"PATH_SEGMENT","start":[29,0,29],"end":[33,0,33],"children":[{"type":"Node","kind":"NAME_REF","start":[29,0,29],"end":[33,0,33],"children":[{"type":"Token","kind":"IDENT","start":[29,0,29],"end":[33,0,33]}]}]}]},{"type":"Node","kind":"ARG_LIST","start":[33,0,33],"end":[39,0,39],"children":[{"type":"Token","kind":"L_PAREN","start":[33,0,33],"end":[34,0,34]},{"type":"Node","kind":"LITERAL","start":[34,0,34],"end":[35,0,35],"children":[{"type":"Token","kind":"INT_NUMBER","start":[34,0,34],"end":[35,0,35]}]},{"type":"Token","kind":"COMMA","start":[35,0,35],"end":[36,0,36]},{"type":"Token","kind":"WHITESPACE","start":[36,0,36],"end":[37,0,37]},{"type":"Node","kind":"LITERAL","start":[37,0,37],"end":[38,0,38],"children":[{"type":"Token","kind":"INT_NUMBER","start":[37,0,37],"end":[38,0,38]}]},{"type":"Token","kind":"R_PAREN","start":[38,0,38],"end":[39,0,39]}]}]},{"type":"Token","kind":"SEMICOLON","start":[39,0,39],"end":[40,0,40]}]},{"type":"Token","kind":"WHITESPACE","start":[40,0,40],"end":[41,0,41]},{"type":"Token","kind":"R_CURLY","start":[41,0,41],"end":[42,0,42]}]}]},{"type":"Token","kind":"WHITESPACE","start":[42,0,42],"end":[43,0,43]},{"type":"Token","kind":"R_CURLY","start":[43,0,43],"end":[44,0,44]}]}]}"#
            ]],
        )
    }
}
