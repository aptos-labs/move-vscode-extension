use crate::ast;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;

impl ast::Path {
    pub fn path_address(&self) -> Option<ast::PathAddress> {
        self.segment().path_address()
    }

    pub fn name_ref(&self) -> Option<ast::NameRef> {
        self.segment().name_ref()
    }

    /** For `Foo::bar::baz::quux` path returns `Foo` */
    pub fn base_path(&self) -> ast::Path {
        let qualifier = self.qualifier();
        if let Some(qualifier) = qualifier {
            qualifier.base_path()
        } else {
            self.clone()
        }
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
