use crate::nameres::namespaces::NsSet;
use crate::{AsName, Name};
use syntax::ast::HasName;
use syntax::{ast, AstNode, SyntaxNode};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScopeEntry {
    pub name: Name,
    pub syntax: SyntaxNode,
    pub ns: NsSet,
}

impl ScopeEntry {
    pub fn from_named<Item>(item: Item, ns: NsSet) -> Option<Self>
    where
        Item: HasName,
    {
        let name = item.name()?;
        Some(ScopeEntry {
            name: name.as_name(),
            syntax: item.syntax().clone(),
            ns,
        })
    }

    pub fn from_name_ref(name_ref: ast::NameRef, ns: NsSet) -> Self {
        ScopeEntry {
            name: name_ref.as_name(),
            syntax: name_ref.syntax().to_owned(),
            ns,
        }
    }
}
