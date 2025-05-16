use crate::ast;

impl ast::ValueArgList {
    pub fn arg_exprs(&self) -> Vec<Option<ast::Expr>> {
        self.args().map(|it| it.expr()).collect()
    }
}
