// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::loc::SyntaxLoc;
use base_db::SourceDatabase;
use std::fmt;
use std::fmt::Formatter;
use syntax::ast;
use syntax::files::InFile;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TyInfer {
    Var(TyVar),
    IntVar(TyIntVar),
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TyVar {
    pub kind: TyVarKind,
}

impl TyVar {
    pub fn new_anonymous(index: usize) -> Self {
        TyVar {
            kind: TyVarKind::Anonymous(index),
        }
    }

    pub fn new_with_origin(origin_loc: SyntaxLoc, index: usize) -> Self {
        TyVar {
            kind: TyVarKind::WithOrigin { origin_loc, index },
        }
    }

    pub fn has_origin(&self) -> bool {
        matches!(self.kind, TyVarKind::WithOrigin { .. })
    }

    pub fn origin_type_param(&self, db: &dyn SourceDatabase) -> Option<InFile<ast::TypeParam>> {
        match &self.kind {
            TyVarKind::WithOrigin { origin_loc, index: _ } => origin_loc.to_ast::<ast::TypeParam>(db),
            _ => None,
        }
    }
}

impl fmt::Debug for TyVar {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.kind {
            TyVarKind::Anonymous(indx) => f.write_str(&format!("?_{}", indx)),
            TyVarKind::WithOrigin { origin_loc, index } => {
                let ident = match origin_loc.node_name() {
                    Some(node_name) => format!("?{node_name}_{index}"),
                    None => format!("?_{index}"),
                };
                f.write_str(&ident)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TyVarKind {
    Anonymous(usize),
    WithOrigin { origin_loc: SyntaxLoc, index: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TyIntVar(usize);

impl TyIntVar {
    pub fn new(index: usize) -> Self {
        TyIntVar(index)
    }
}
