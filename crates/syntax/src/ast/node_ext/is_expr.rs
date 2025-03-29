use crate::ast;

impl ast::IsExpr {
    pub fn path_types(&self) -> Vec<ast::PathType> {
        self.types().filter_map(|t| t.path_type()).collect()
    }
}
