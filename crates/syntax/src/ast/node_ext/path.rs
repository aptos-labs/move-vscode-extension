use crate::ast;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;

impl ast::Path {
    pub fn path_address(&self) -> Option<ast::PathAddress> {
        self.segment().unwrap().path_address()
    }

    pub fn name_ref(&self) -> Option<ast::NameRef> {
        self.segment().unwrap().name_ref()
    }

    /// for `Foo::bar` in `Foo::bar::baz::quux` returns `Foo::bar::baz::quux`
    pub fn root_path(&self) -> ast::Path {
        let parent_path = self.syntax.parent_of_type::<ast::Path>();
        if parent_path.is_some() {
            parent_path.unwrap().root_path()
        } else {
            self.clone()
        }
    }

    pub fn use_speck(&self) -> Option<ast::UseSpeck> {
        self.root_path().syntax.parent_of_type::<ast::UseSpeck>()
    }
}
