// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::loc::{SyntaxLoc, SyntaxLocFileExt};
use crate::types::abilities::Ability;
use base_db::SourceDatabase;
use syntax::ast;
use syntax::files::InFile;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct TyTypeParameter {
    pub origin_loc: SyntaxLoc,
}

impl TyTypeParameter {
    pub fn new(origin: InFile<ast::TypeParam>) -> Self {
        TyTypeParameter { origin_loc: origin.loc() }
    }

    pub fn from_loc(origin_loc: SyntaxLoc) -> Self {
        TyTypeParameter { origin_loc }
    }

    pub fn origin_type_param(&self, db: &dyn SourceDatabase) -> Option<InFile<ast::TypeParam>> {
        self.origin_loc.to_ast::<ast::TypeParam>(db)
    }

    pub fn abilities(&self, db: &dyn SourceDatabase) -> Option<Vec<Ability>> {
        let type_param = self.origin_type_param(db)?;
        let abilities = type_param
            .value
            .ability_bounds()
            .into_iter()
            .filter_map(|it| Ability::from_ast(&it))
            .collect::<Vec<_>>();
        Some(abilities)
    }
}
