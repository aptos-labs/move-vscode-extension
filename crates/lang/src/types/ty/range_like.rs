// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::types::fold::{TypeFoldable, TypeFolder, TypeVisitor};
use crate::types::ty::Ty;
use std::ops::Deref;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TySequence {
    Vector(Box<Ty>),
    Range(Box<Ty>),
}

impl TySequence {
    pub fn item(&self) -> Ty {
        match self {
            TySequence::Vector(ty) => ty.deref().to_owned(),
            TySequence::Range(ty) => ty.deref().to_owned(),
        }
    }
}

impl TypeFoldable<TySequence> for TySequence {
    fn deep_fold_with(self, folder: impl TypeFolder) -> TySequence {
        match self {
            TySequence::Vector(item_ty) => {
                let item_ty = item_ty.deref().to_owned();
                TySequence::Vector(Box::new(folder.fold_ty(item_ty)))
            }
            TySequence::Range(item_ty) => {
                let item_ty = item_ty.deref().to_owned();
                TySequence::Range(Box::new(folder.fold_ty(item_ty)))
            }
        }
    }

    fn deep_visit_with(&self, visitor: impl TypeVisitor) -> bool {
        visitor.visit_ty(&self.item())
    }
}

impl From<TySequence> for Ty {
    fn from(value: TySequence) -> Self {
        Ty::Seq(value)
    }
}

// #[derive(Debug, Clone, Eq, PartialEq)]
// pub struct TyVectorLike {
//     pub item: Box<Ty>,
//     pub kind: VectorKind,
// }

// #[derive(Debug, Clone, Eq, PartialEq)]
// pub enum VectorKind {
//     Vector,
//     Range,
// }
