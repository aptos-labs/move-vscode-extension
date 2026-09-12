use crate::engine::fmt_model::{FmtBlock, IndentType};
use crate::rules;
use syntax::SyntaxKind::{BIN_EXPR, L_CURLY, WHITESPACE};
use syntax::{AstNode, NodeOrToken, SyntaxElement, SyntaxKind, SyntaxNode, ast};

impl FmtBlock {
    pub(crate) fn build(
        syntax_element: SyntaxElement,
        ws_before: String,
        indent_type: IndentType,
    ) -> FmtBlock {
        let children = match &syntax_element {
            SyntaxElement::Node(node)
                if node.kind() == BIN_EXPR
                    && let Some(op_bp) = bin_op_bp(node)
                    && is_root_of_bin_chain(node) =>
            {
                FmtBlock::build_children_for_bin_chain_block(node, op_bp)
            }
            SyntaxElement::Node(_) | SyntaxElement::Token(_) => {
                FmtBlock::build_children_for_syntax_block(&syntax_element, syntax_element.kind())
            }
        };

        FmtBlock::new(syntax_element, indent_type, ws_before, children)
    }

    fn build_children_for_syntax_block(
        syntax_element: &SyntaxElement,
        parent_kind: SyntaxKind,
    ) -> Vec<FmtBlock> {
        match syntax_element {
            SyntaxElement::Token(_) => Vec::new(),
            SyntaxElement::Node(node) => {
                let mut children = Vec::new();
                let mut after_l_curly = false;
                FmtBlock::build_children(&mut children, node, None, |children, child, ws| {
                    let child_indent_type =
                        rules::indent::get_indent_type(parent_kind, child.kind(), after_l_curly);
                    if child.kind() == L_CURLY {
                        after_l_curly = true;
                    }
                    children.push(FmtBlock::build(child, ws.unwrap_or_default(), child_indent_type));
                });
                children
            }
        }
    }

    fn build_children(
        children: &mut Vec<FmtBlock>,
        node: &SyntaxNode,
        leading_ws: Option<String>,
        mut push_child: impl FnMut(&mut Vec<FmtBlock>, SyntaxElement, Option<String>),
    ) {
        let mut pending_ws = leading_ws;

        for child_element in node.children_with_tokens() {
            if child_element.kind() == WHITESPACE {
                pending_ws = Some(child_element.as_token().unwrap().text().to_string());
                continue;
            }

            let ws = pending_ws.take();
            push_child(children, child_element, ws);
        }
    }

    fn append_bin_chain_children(
        children: &mut Vec<FmtBlock>,
        node: &SyntaxNode,
        op_bp: u8,
        leading_ws: Option<String>,
    ) {
        FmtBlock::build_children(children, node, leading_ws, |children, child, ws| {
            if let Some(child_node) = child.as_node()
                && child_node.kind() == BIN_EXPR
                && bin_op_bp(child_node) == Some(op_bp)
            {
                FmtBlock::append_bin_chain_children(children, child_node, op_bp, ws);
                return;
            }

            let child_indent_type = rules::indent::get_indent_type(BIN_EXPR, child.kind(), false);
            children.push(FmtBlock::build(child, ws.unwrap_or_default(), child_indent_type));
        });
    }

    fn build_children_for_bin_chain_block(node: &SyntaxNode, op_bp: u8) -> Vec<FmtBlock> {
        let mut children = Vec::new();
        FmtBlock::append_bin_chain_children(&mut children, node, op_bp, None);
        children
    }
}

fn is_root_of_bin_chain(node: &SyntaxNode) -> bool {
    debug_assert_eq!(node.kind(), BIN_EXPR);
    match node.parent() {
        None => true,
        Some(parent) if parent.kind() != BIN_EXPR => true,
        Some(parent) => bin_op_bp(&parent) != bin_op_bp(node),
    }
}

fn bin_op_bp(node: &SyntaxNode) -> Option<u8> {
    ast::BinExpr::cast(node.clone())?.op_bp()
}
