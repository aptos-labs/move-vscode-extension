use crate::ast;
use crate::ast::TypeParam;

impl ast::GenericSpecStmt {
    pub fn type_params(&self) -> Vec<TypeParam> {
        match self {
            ast::GenericSpecStmt::AxiomStmt(stmt) => stmt.type_params(),
            ast::GenericSpecStmt::InvariantStmt(stmt) => stmt.type_params(),
        }
    }
}
