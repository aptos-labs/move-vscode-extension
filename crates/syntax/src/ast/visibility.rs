// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast::support;
use crate::{AstNode, ast};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VisLevel {
    Friend,
    Package,
}

impl Display for VisLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VisLevel::Friend => write!(f, "friend"),
            VisLevel::Package => write!(f, "package"),
        }
    }
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
