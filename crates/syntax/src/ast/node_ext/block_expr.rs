use crate::ast::HasStmts;
use crate::parse::SyntaxKind::{FOR_EXPR, FUN, IF_EXPR, LOOP_EXPR, WHILE_EXPR};
use crate::{AstNode, ast};

impl ast::BlockExpr {
    pub fn schema_fields(&self) -> Vec<ast::SchemaField> {
        self.stmts().filter_map(|it| it.schema_field()).collect()
    }

    pub fn spec_inline_functions(&self) -> Vec<ast::SpecInlineFun> {
        self.stmts().filter_map(|it| it.spec_inline_fun()).collect()
    }

    /// ```not_rust
    /// fn foo() { not_stand_alone }
    /// const FOO: () = { stand_alone };
    /// ```
    pub fn is_standalone(&self) -> bool {
        let parent = match self.syntax().parent() {
            Some(it) => it,
            None => return true,
        };
        match parent.kind() {
            FOR_EXPR | IF_EXPR => parent
                .children()
                .find(|it| ast::Expr::can_cast(it.kind()))
                .is_none_or(|it| it == *self.syntax()),
            FUN | WHILE_EXPR | LOOP_EXPR => false,
            _ => true,
        }
    }
}
