// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::loc::SyntaxLoc;
use crate::types::fold::{TypeFoldable, TypeFolder, TypeVisitor};
use crate::types::ty::Ty;
use base_db::SourceDatabase;
use std::iter;
use std::ops::Deref;
use syntax::ast;
use syntax::files::InFile;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TyCallable {
    pub param_types: Vec<Ty>,
    pub ret_type: Box<Ty>,
    pub kind: CallableKind,
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

pub enum Callable {
    Fun(InFile<ast::AnyFun>),
    LambdaExpr(InFile<ast::LambdaExpr>),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CallableKind {
    Lambda(Option<SyntaxLoc>),
    Fun(Option<SyntaxLoc>),
}

impl CallableKind {
    pub fn callable(&self, db: &dyn SourceDatabase) -> Option<Callable> {
        match self {
            CallableKind::Fun(Some(fun_loc)) => {
                let fun = fun_loc.to_ast::<ast::AnyFun>(db)?;
                Some(Callable::Fun(fun))
            }
            CallableKind::Lambda(Some(lambda_expr_loc)) => {
                let lambda_expr = lambda_expr_loc.to_ast::<ast::LambdaExpr>(db)?;
                Some(Callable::LambdaExpr(lambda_expr))
            }
            _ => None,
        }
    }
}

impl TyCallable {
    pub fn new(param_types: Vec<Ty>, ret_type: Ty, kind: CallableKind) -> Self {
        TyCallable {
            param_types,
            ret_type: Box::new(ret_type),
            kind,
        }
    }

    pub fn fake(n_params: usize, kind: CallableKind) -> Self {
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
