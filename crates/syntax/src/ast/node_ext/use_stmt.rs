use crate::ast;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;

impl ast::UseStmt {
    pub fn owner(&self) -> Option<ast::AnyUseStmtsOwner> {
        self.syntax.parent_of_type()
    }

    pub fn group_use_specks(&self) -> Vec<ast::UseSpeck> {
        self.use_speck()
            .and_then(|it| it.use_group())
            .map(|it| it.use_specks().collect())
            .unwrap_or_default()
    }
}
