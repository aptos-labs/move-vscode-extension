use crate::ast;

impl ast::BehaviorPredicateExpr {
    pub fn fun_path_type(&self) -> Option<ast::PathType> {
        let first_type_arg = self.type_arg_list()?.type_args().into_iter().next()?;
        first_type_arg.type_()?.path_type()
    }

    pub fn fun_path(&self) -> Option<ast::Path> {
        self.fun_path_type().map(|it| it.path())
    }

    pub fn arg_exprs(&self) -> Vec<Option<ast::Expr>> {
        self.value_arg_list().map(|it| it.arg_exprs()).unwrap_or_default()
    }

    pub fn predicate_kind(&self) -> BehaviorPredicateKind {
        match self.ident_token().text() {
            "aborts_of" => BehaviorPredicateKind::AbortsOf,
            "requires_of" => BehaviorPredicateKind::RequiresOf,
            "ensures_of" => BehaviorPredicateKind::EnsuresOf,
            "result_of" => BehaviorPredicateKind::ResultOf,
            "folds_of" => BehaviorPredicateKind::FoldsOf,
            "unchanged_of" => BehaviorPredicateKind::UnchangedOf,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum BehaviorPredicateKind {
    AbortsOf,
    RequiresOf,
    EnsuresOf,
    ResultOf,
    FoldsOf,
    UnchangedOf,
}
