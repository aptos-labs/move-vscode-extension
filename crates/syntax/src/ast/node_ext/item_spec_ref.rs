use crate::ast;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;

impl ast::ItemSpecRef {
    pub fn item_spec(&self) -> ast::ItemSpec {
        self.syntax
            .parent_of_type::<ast::ItemSpec>()
            .expect("unreachable")
    }
}
