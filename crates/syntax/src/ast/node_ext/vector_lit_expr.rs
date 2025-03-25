use crate::ast;

impl ast::VectorLitExpr {
    pub fn type_arg(&self) -> Option<ast::TypeArg> {
        self.type_arg_list()?.type_arguments().next()
    }
}
