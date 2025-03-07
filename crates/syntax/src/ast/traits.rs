mod has_item_list;

use crate::ast::{support, AstChildren, StmtList, TypeParam, TypeParamList};
use crate::{ast, AstNode};

pub use has_item_list::HasItemList;

pub trait HasName: AstNode {
    fn name(&self) -> Option<ast::Name> {
        support::child(self.syntax())
    }
}

pub trait HasStmtList: AstNode {
    fn stmt_list(&self) -> Option<StmtList> {
        support::child(&self.syntax())
    }
    fn stmts(&self) -> impl Iterator<Item = ast::Stmt> {
        self.stmt_list().into_iter().flat_map(|it| it.statements())
    }
    fn let_stmts(&self) -> impl Iterator<Item = ast::LetStmt> {
        self.stmts().filter_map(|it| it.let_stmt())
    }
    fn tail_expr(&self) -> Option<ast::Expr> {
        self.stmt_list()?.tail_expr()
    }
}

pub trait HasTypeParams: AstNode {
    fn type_param_list(&self) -> Option<TypeParamList> {
        support::child(&self.syntax())
    }

    fn type_params(&self) -> Vec<TypeParam> {
        self.type_param_list()
            .map(|l| l.type_parameters().collect())
            .unwrap_or_default()
    }
}

pub trait HasAttrs: AstNode {
    fn attrs(&self) -> AstChildren<ast::Attr> {
        support::children(self.syntax())
    }
}