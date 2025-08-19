// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::loc::{SyntaxLoc, SyntaxLocFileExt};
use crate::types::fold::{TypeFoldable, TypeFolder, TypeVisitor};
use crate::types::has_type_params_ext::GenericItemExt;
use crate::types::substitution::Substitution;
use crate::types::ty::Ty;
use base_db::SourceDatabase;
use syntax::ast;
use syntax::files::InFile;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TyAdt {
    pub adt_item_loc: SyntaxLoc,
    pub substitution: Substitution,
    pub type_args: Vec<Ty>,
}

impl TyAdt {
    pub fn new(item: InFile<ast::StructOrEnum>) -> Self {
        TyAdt {
            adt_item_loc: item.loc(),
            substitution: item.ty_type_params_subst(),
            type_args: item
                .ty_type_params()
                .into_iter()
                .map(|it| Ty::TypeParam(it))
                .collect(),
        }
    }

    pub fn adt_item(&self, db: &dyn SourceDatabase) -> Option<InFile<ast::StructOrEnum>> {
        self.adt_item_loc.to_ast::<ast::StructOrEnum>(db)
    }

    pub fn adt_item_module(&self, db: &dyn SourceDatabase) -> Option<ast::Module> {
        let adt_item = self.adt_item(db)?;
        let m = adt_item.value.module();
        Some(m)
    }
}

impl TypeFoldable<TyAdt> for TyAdt {
    fn deep_fold_with(self, folder: &impl TypeFolder) -> TyAdt {
        TyAdt {
            adt_item_loc: self.adt_item_loc,
            substitution: self.substitution.deep_fold_with(folder),
            type_args: folder.fold_tys(self.type_args),
        }
    }

    fn deep_visit_with(&self, visitor: impl TypeVisitor) -> bool {
        self.substitution.deep_visit_with(visitor.clone()) || visitor.visit_tys(&self.type_args)
    }
}

impl From<TyAdt> for Ty {
    fn from(value: TyAdt) -> Self {
        Ty::Adt(value)
    }
}
