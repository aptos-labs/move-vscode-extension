use crate::ast;

impl ast::BehaviorPredicateExpr {
    pub fn fun_path(&self) -> Option<ast::Path> {
        let first_type_arg = self.type_arg_list()?.type_args().into_iter().next()?;
        let path_type = first_type_arg.type_()?.path_type()?;
        Some(path_type.path())
    }
}
