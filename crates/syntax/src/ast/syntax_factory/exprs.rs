// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast::syntax_factory::{SyntaxFactory, ast_from_text};
use crate::syntax_editor::mapping::SyntaxMappingBuilder;
use crate::{AstNode, ast};

impl SyntaxFactory {
    pub fn bin_expr(&self, lhs: ast::Expr, op: ast::BinaryOp, rhs: ast::Expr) -> ast::BinExpr {
        let bin_expr = expr_from_text::<ast::BinExpr>(&format!("{lhs} {op} {rhs}")).clone_for_update();
        if let Some(mut mapping) = self.mappings() {
            let mut builder = SyntaxMappingBuilder::new(bin_expr.syntax().clone());
            builder.map_node(lhs.syntax().clone(), bin_expr.lhs().unwrap().syntax().clone());
            builder.map_node(rhs.syntax().clone(), bin_expr.rhs().unwrap().syntax().clone());
            builder.finish(&mut mapping);
        }
        bin_expr
    }

    pub fn index_expr(&self, base_expr: ast::Expr, arg_expr: ast::Expr) -> ast::IndexExpr {
        let index_expr =
            expr_from_text::<ast::IndexExpr>(&format!("{base_expr}[{arg_expr}]")).clone_for_update();
        if let Some(mut mapping) = self.mappings() {
            let mut builder = SyntaxMappingBuilder::new(index_expr.syntax().clone());
            builder.map_node(
                base_expr.syntax().clone(),
                index_expr.base_expr().syntax().clone(),
            );
            builder.map_node(
                arg_expr.syntax().clone(),
                index_expr.arg_expr().unwrap().syntax().clone(),
            );
            builder.finish(&mut mapping);
        }
        index_expr
    }

    pub fn paren_expr(&self, expr: ast::Expr) -> ast::ParenExpr {
        let paren_expr = expr_from_text::<ast::ParenExpr>(&format!("({expr})")).clone_for_update();
        if let Some(mut mapping) = self.mappings() {
            let mut builder = SyntaxMappingBuilder::new(paren_expr.syntax().clone());
            builder.map_node(expr.syntax().clone(), paren_expr.expr().unwrap().syntax().clone());
            builder.finish(&mut mapping);
        }
        paren_expr
    }

    pub fn method_call_expr(
        &self,
        receiver: ast::Expr,
        name_ref: ast::NameRef,
        type_arg_list: Option<ast::TypeArgList>,
        value_arg_list: ast::ValueArgList,
    ) -> ast::MethodCallExpr {
        let type_arg_list_with_colon_colon = type_arg_list
            .as_ref()
            .map(|it| format!("::{it}"))
            .unwrap_or("".to_string());
        let method_call_expr = expr_from_text::<ast::MethodCallExpr>(&format!(
            "{receiver}.{name_ref}{type_arg_list_with_colon_colon}{value_arg_list}"
        ))
        .clone_for_update();
        if let Some(mut mapping) = self.mappings() {
            let mut builder = SyntaxMappingBuilder::new(method_call_expr.syntax().clone());
            builder.map_node(
                receiver.syntax().clone(),
                method_call_expr.receiver_expr().syntax().clone(),
            );
            builder.map_node(
                name_ref.syntax().clone(),
                method_call_expr.name_ref().unwrap().syntax().clone(),
            );
            if let Some(type_arg_list) = type_arg_list {
                builder.map_node(
                    type_arg_list.syntax().clone(),
                    method_call_expr.type_arg_list().unwrap().syntax().clone(),
                );
            }
            builder.map_node(
                value_arg_list.syntax().clone(),
                method_call_expr.value_arg_list().unwrap().syntax().clone(),
            );
            builder.finish(&mut mapping);
        }
        method_call_expr
    }
}

pub(super) fn expr_from_text<E: Into<ast::Expr> + AstNode>(text: &str) -> E {
    let block_expr =
        ast_from_text::<ast::BlockExpr>(&format!("module 0x1::m {{ fun main() {{ {text} }} }}"));
    E::cast(block_expr.tail_expr().unwrap().syntax().clone()).unwrap()
}
