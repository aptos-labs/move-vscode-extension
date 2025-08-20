// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;
use crate::ast::{HasAttrs, HasVisibility};

impl ast::AnyFun {
    pub fn is_native(&self) -> bool {
        match self {
            ast::AnyFun::Fun(fun) => fun.native_token().is_some(),
            ast::AnyFun::SpecFun(fun) => fun.native_token().is_some(),
            ast::AnyFun::SpecInlineFun(fun) => fun.native_token().is_some(),
        }
    }

    pub fn is_uninterpreted(&self) -> bool {
        match self {
            ast::AnyFun::Fun(fun) => false,
            ast::AnyFun::SpecFun(fun) => fun.spec_block().is_some(),
            ast::AnyFun::SpecInlineFun(fun) => fun.spec_block().is_some(),
        }
    }

    pub fn params(&self) -> Vec<ast::Param> {
        self.param_list()
            .map(|list| list.params().collect())
            .unwrap_or_default()
    }

    pub fn params_as_bindings(&self) -> Vec<ast::IdentPat> {
        self.params()
            .into_iter()
            .filter_map(|param| param.ident_pat())
            .collect()
    }

    pub fn return_type(&self) -> Option<ast::Type> {
        self.ret_type()?.type_()
    }

    pub fn block_expr(&self) -> Option<ast::BlockExpr> {
        match self {
            ast::AnyFun::Fun(fun) => fun.body(),
            ast::AnyFun::SpecFun(fun) => fun.spec_block(),
            ast::AnyFun::SpecInlineFun(fun) => fun.spec_block(),
        }
    }

    pub fn to_generic_element(&self) -> ast::GenericElement {
        match self.clone() {
            ast::AnyFun::Fun(it) => it.into(),
            ast::AnyFun::SpecFun(it) => it.into(),
            ast::AnyFun::SpecInlineFun(it) => it.into(),
        }
    }
}

impl ast::Fun {
    pub fn to_any_fun(&self) -> ast::AnyFun {
        ast::AnyFun::Fun(self.clone())
    }
}

impl ast::SpecFun {
    pub fn to_any_fun(&self) -> ast::AnyFun {
        ast::AnyFun::SpecFun(self.clone())
    }
}

impl ast::SpecInlineFun {
    pub fn to_any_fun(&self) -> ast::AnyFun {
        ast::AnyFun::SpecInlineFun(self.clone())
    }
}
