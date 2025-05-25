use crate::ast;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;

impl ast::ItemSpecTypeParam {
    pub fn item_spec_type_param_list(&self) -> ast::ItemSpecTypeParamList {
        self.syntax
            .parent_of_type::<ast::ItemSpecTypeParamList>()
            .unwrap()
    }
    pub fn item_spec(&self) -> ast::ItemSpec {
        self.item_spec_type_param_list()
            .syntax
            .parent_of_type::<ast::ItemSpec>()
            .unwrap()
    }
}
