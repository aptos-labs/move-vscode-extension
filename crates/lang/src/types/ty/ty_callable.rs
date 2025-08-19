// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::loc::SyntaxLoc;
use crate::types::fold::{TypeFoldable, TypeFolder, TypeVisitor};
use crate::types::substitution::Substitution;
use crate::types::ty::Ty;
use std::iter;
use std::ops::Deref;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TyCallable {
    pub param_types: Vec<Ty>,
    pub ret_type: Box<Ty>,
    pub kind: TyCallableKind,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TyCallableKind {
    Named(Substitution, Option<SyntaxLoc>),
    Lambda(Option<SyntaxLoc>),
}

impl TyCallableKind {
    pub fn named(subst: Substitution, loc: Option<SyntaxLoc>) -> Self {
        TyCallableKind::Named(subst, loc)
    }

    pub fn fake() -> Self {
        TyCallableKind::Named(Substitution::default(), None)
    }
}

impl From<TyCallable> for Ty {
    fn from(value: TyCallable) -> Self {
        Ty::Callable(value)
    }
}

impl TyCallable {
    pub fn ret_type(&self) -> Ty {
        self.ret_type.deref().to_owned()
    }
}

impl TyCallable {
    pub fn new(param_types: Vec<Ty>, ret_type: Ty, kind: TyCallableKind) -> Self {
        TyCallable {
            param_types,
            ret_type: Box::new(ret_type),
            kind,
        }
    }

    pub fn fake(n_params: usize, kind: TyCallableKind) -> Self {
        TyCallable {
            param_types: iter::repeat_n(Ty::Unknown, n_params).collect(),
            ret_type: Box::new(Ty::Unknown),
            kind,
        }
    }
}

impl TypeFoldable<TyCallableKind> for TyCallableKind {
    fn deep_fold_with(self, folder: &impl TypeFolder) -> TyCallableKind {
        match self {
            TyCallableKind::Named(subst, loc) => TyCallableKind::Named(subst.fold_with(folder), loc),
            TyCallableKind::Lambda(loc) => TyCallableKind::Lambda(loc),
        }
    }

    fn deep_visit_with(&self, visitor: impl TypeVisitor) -> bool {
        match self {
            TyCallableKind::Named(subst, _) => subst.visit_with(visitor),
            TyCallableKind::Lambda(_) => false,
        }
    }
}

impl TypeFoldable<TyCallable> for TyCallable {
    fn deep_fold_with(self, folder: &impl TypeFolder) -> TyCallable {
        let TyCallable { param_types, ret_type, kind } = self;
        TyCallable::new(
            folder.fold_tys(param_types),
            folder.fold_ty(*ret_type),
            kind.fold_with(folder),
        )
    }

    fn deep_visit_with(&self, visitor: impl TypeVisitor) -> bool {
        visitor.visit_tys(&self.param_types)
            || visitor.visit_ty(&self.ret_type)
            || self.kind.visit_with(visitor)
    }
}
