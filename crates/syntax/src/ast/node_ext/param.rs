use crate::ast;
use crate::ast::NamedElement;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;

impl ast::Param {
    pub fn param_list(&self) -> Option<ast::ParamList> {
        self.syntax.parent_of_type::<ast::ParamList>()
    }

    pub fn any_fun(&self) -> Option<ast::AnyFun> {
        self.param_list()?.syntax.parent_of_type::<ast::AnyFun>()
    }

    pub fn ident_name(&self) -> String {
        if self.wildcard_pat().is_some() {
            return "_".to_string();
        }
        // todo: ident_pat can be none
        let ident_pat = self.ident_pat().unwrap();
        ident_pat.name().unwrap().as_string()
    }
}
