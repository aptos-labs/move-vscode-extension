// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::Semantics;
use base_db::SourceDatabase;
use syntax::ast;
use syntax::files::{InFile, InFileExt};

pub enum CalleeKind {
    Function(ast::AnyFun),
    TupleStruct(ast::Struct),
    TupleEnumVariant(ast::Variant),
    AssertMacro,
    // Lambda(ast::LambdaType),
}

pub fn callee_kind<DB: SourceDatabase>(
    sema: &Semantics<'_, DB>,
    callable: &InFile<ast::AnyCallExpr>,
) -> Option<InFile<CalleeKind>> {
    let (call_file_id, callable) = callable.unpack_ref();
    let callee = match callable {
        ast::AnyCallExpr::CallExpr(call_expr) => {
            let reference = call_expr.path()?.reference();
            sema.resolve_to_element::<ast::NamedElement>(reference.in_file(call_file_id))
        }
        ast::AnyCallExpr::MethodCallExpr(method_call_expr) => {
            sema.resolve_to_element::<ast::NamedElement>(method_call_expr.clone().in_file(call_file_id))
        }
        ast::AnyCallExpr::AssertMacroExpr(_) => {
            return Some(CalleeKind::AssertMacro.in_file(call_file_id));
        }
    }?;

    let (file_id, callee) = callee.unpack();
    let kind = match callee {
        ast::NamedElement::Fun(fun) => CalleeKind::Function(fun.into()),
        ast::NamedElement::SpecFun(fun) => CalleeKind::Function(fun.into()),
        ast::NamedElement::SpecInlineFun(fun) => CalleeKind::Function(fun.into()),
        ast::NamedElement::Struct(s) => CalleeKind::TupleStruct(s),
        ast::NamedElement::Variant(v) => CalleeKind::TupleEnumVariant(v),
        _ => {
            return None;
        }
    };

    Some(kind.in_file(file_id))
}
