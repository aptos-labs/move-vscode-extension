use crate::ast::support;
use crate::{AstNode, ast};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VisLevel {
    Friend,
    Package,
    // Script,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Vis {
    Public,
    Restricted(VisLevel),
    Private,
}

pub trait HasVisibility: AstNode {
    fn vis(&self) -> Vis {
        let Some(vis_modifier) = support::child::<ast::VisibilityModifier>(self.syntax()) else {
            return Vis::Private;
        };

        if vis_modifier.is_public_friend() || vis_modifier.is_friend() {
            return Vis::Restricted(VisLevel::Friend);
        }
        if vis_modifier.is_public_package() || vis_modifier.is_package() {
            return Vis::Restricted(VisLevel::Package);
        }
        if vis_modifier.is_public_script() {
            // `public(script)` considered public
            return Vis::Public;
        }
        if let Some(fun) = ast::Fun::cast(self.syntax().clone()) {
            // `entry` considered public
            if fun.is_entry() {
                return Vis::Public;
            }
        }
        Vis::Public
    }
}
