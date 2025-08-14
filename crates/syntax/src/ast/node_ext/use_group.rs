use crate::ast;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;

impl ast::UseGroup {
    pub fn parent_use_speck(&self) -> ast::UseSpeck {
        self.syntax
            .parent_of_type::<ast::UseSpeck>()
            .expect("always exists")
    }

    pub fn use_stmt(&self) -> Option<ast::UseStmt> {
        self.parent_use_speck().use_stmt()
    }
}
