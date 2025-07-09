// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::loc::SyntaxLoc;
use crate::types::fold::{TypeFoldable, TypeFolder, TypeVisitor};
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
    Named(Option<SyntaxLoc>),
    Lambda(Option<SyntaxLoc>),
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

impl TypeFoldable<TyCallable> for TyCallable {
    fn deep_fold_with(self, folder: impl TypeFolder) -> TyCallable {
        let TyCallable { param_types, ret_type, kind } = self;
        TyCallable::new(folder.fold_tys(param_types), folder.fold_ty(*ret_type), kind)
    }

    fn deep_visit_with(&self, visitor: impl TypeVisitor) -> bool {
        visitor.visit_tys(&self.param_types) || visitor.visit_ty(&self.ret_type)
    }
}
