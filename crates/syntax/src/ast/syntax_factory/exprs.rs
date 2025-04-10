use std::fmt::format;
use crate::ast::syntax_factory::{ast_from_text, SyntaxFactory};
use crate::{ast, AstNode};

impl SyntaxFactory {
    pub fn expr_paren(&self, expr: ast::Expr) -> ast::Expr {
        expr_from_text(&format!("({expr})"))
    }

    pub fn expr_method_call(
        &self,
        receiver: ast::Expr,
        name_ref: ast::NameRef,
        type_arg_list: Option<ast::TypeArgList>,
        arg_list: ast::ArgList,
    ) -> ast::Expr {
        let type_arg_list = type_arg_list.map(|it| format!("::{it}")).unwrap_or("".to_string());
        expr_from_text(&format!("{receiver}.{name_ref}{type_arg_list}{arg_list}"))
    }
}

pub(super) fn expr_from_text<E: Into<ast::Expr> + AstNode>(text: &str) -> E {
    let block_expr =
        ast_from_text::<ast::BlockExpr>(&format!("module 0x1::m {{ fun main() {{ {text} }} }}"));
    E::cast(block_expr.tail_expr().unwrap().syntax().clone()).unwrap()
}
