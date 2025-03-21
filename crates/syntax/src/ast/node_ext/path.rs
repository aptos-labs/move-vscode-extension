use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::{ast, AstNode};

impl ast::Path {
    pub fn path_address(&self) -> Option<ast::PathAddress> {
        self.segment().path_address()
    }

    pub fn name_ref(&self) -> Option<ast::NameRef> {
        self.segment().name_ref()
    }

    pub fn as_reference(&self) -> ast::AnyReferenceElement {
        ast::AnyReferenceElement::cast(self.syntax.to_owned()).unwrap()
    }

    pub fn type_args(&self) -> Vec<ast::TypeArg> {
        self.segment()
            .type_arg_list()
            .map(|it| it.type_arguments().collect())
            .unwrap_or_default()
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

    pub fn is_use_speck(&self) -> bool {
        self.use_speck().is_some()
    }
}
