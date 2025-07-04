// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::types::fold::{TypeFoldable, TypeFolder, TypeVisitor};
use crate::types::ty::Ty;
use std::iter;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TyTuple {
    pub types: Vec<Ty>,
}

impl TyTuple {
    pub fn new(types: Vec<Ty>) -> Self {
        TyTuple { types }
    }

    pub fn unknown(arity: usize) -> Self {
        TyTuple::new(iter::repeat_n(Ty::Unknown, arity).collect())
    }
}

impl TypeFoldable<TyTuple> for TyTuple {
    fn deep_fold_with(self, folder: impl TypeFolder) -> TyTuple {
        TyTuple {
            types: folder.fold_tys(self.types),
        }
    }

    fn deep_visit_with(&self, visitor: impl TypeVisitor) -> bool {
        visitor.visit_tys(&self.types)
    }
}
