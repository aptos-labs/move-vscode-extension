use crate::ast::support;
use crate::{ast, AstNode};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VisLevel {
    Friend,
    Package,
    // Script,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Vis {
    Public,
    Restricted(VisLevel),
    Private,
}

pub trait HasVisibility: AstNode {
    fn visibility(&self) -> Option<ast::Visibility> {
        support::child(self.syntax())
    }
}

impl ast::Fun {
    pub fn vis(&self) -> Vis {
        let Some(visibility) = self.visibility() else {
            return Vis::Private;
        };
        if visibility.is_public_friend() || visibility.is_friend() {
            return Vis::Restricted(VisLevel::Friend);
        }
        if visibility.is_public_package() || visibility.is_package() {
            return Vis::Restricted(VisLevel::Package);
        }
        if visibility.is_public_script() || self.is_entry() {
            return Vis::Public;
        }
        Vis::Public
    }
}
