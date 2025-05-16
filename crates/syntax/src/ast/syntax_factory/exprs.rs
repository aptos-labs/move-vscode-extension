use crate::ast::syntax_factory::{ast_from_text, SyntaxFactory};
use crate::syntax_editor::mapping::SyntaxMappingBuilder;
use crate::{ast, AstNode};
use std::fmt::format;

impl SyntaxFactory {
    pub fn expr_bin(&self, lhs: ast::Expr, op: ast::BinaryOp, rhs: ast::Expr) -> ast::BinExpr {
        let ast::Expr::BinExpr(ast) = expr_bin_op(lhs.clone(), op, rhs.clone()).clone_for_update()
        else {
            unreachable!()
        };

        if let Some(mut mapping) = self.mappings() {
            let mut builder = SyntaxMappingBuilder::new(ast.syntax().clone());
            builder.map_node(lhs.syntax().clone(), ast.lhs().unwrap().syntax().clone());
            builder.map_node(rhs.syntax().clone(), ast.rhs().unwrap().syntax().clone());
            builder.finish(&mut mapping);
        }

        ast
    }

    pub fn expr_paren(&self, expr: ast::Expr) -> ast::Expr {
        expr_from_text(&format!("({expr})"))
    }

    pub fn expr_method_call(
        &self,
        receiver: ast::Expr,
        name_ref: ast::NameRef,
        type_arg_list: Option<ast::TypeArgList>,
        arg_list: ast::ValueArgList,
    ) -> ast::Expr {
        let type_arg_list = type_arg_list
            .map(|it| format!("::{it}"))
            .unwrap_or("".to_string());
        expr_from_text(&format!("{receiver}.{name_ref}{type_arg_list}{arg_list}"))
    }
}

// Consider `op: SyntaxKind` instead for nicer syntax at the call-site?
pub(super) fn expr_bin_op(lhs: ast::Expr, op: ast::BinaryOp, rhs: ast::Expr) -> ast::Expr {
    expr_from_text(&format!("{lhs} {op} {rhs}"))
}

pub(super) fn expr_from_text<E: Into<ast::Expr> + AstNode>(text: &str) -> E {
    let block_expr =
        ast_from_text::<ast::BlockExpr>(&format!("module 0x1::m {{ fun main() {{ {text} }} }}"));
    E::cast(block_expr.tail_expr().unwrap().syntax().clone()).unwrap()
}
