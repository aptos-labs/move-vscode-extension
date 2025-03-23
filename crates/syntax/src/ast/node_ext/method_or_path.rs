use crate::ast;

impl ast::MethodOrPath {
    pub fn type_arg_list(&self) -> Option<ast::TypeArgList> {
        match self {
            ast::MethodOrPath::MethodCallExpr(method_call_expr) => method_call_expr.type_arg_list(),
            ast::MethodOrPath::Path(path) => path.segment().type_arg_list(),
        }
    }
}
