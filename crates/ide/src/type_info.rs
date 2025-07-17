// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use ide_db::RootDatabase;
use lang::Semantics;
use lang::types::ty::Ty;
use syntax::files::{FilePosition, InFileExt};
use syntax::{AstNode, algo, ast};

pub(crate) fn expr_type_info(
    db: &RootDatabase,
    FilePosition { file_id, offset }: FilePosition,
) -> Option<String> {
    let sema = Semantics::new(db, file_id);

    let file = sema.parse(file_id);

    let expr = algo::find_node_at_offset::<ast::Expr>(file.syntax(), offset)?;
    let expr_ty = sema.get_expr_type(&expr.in_file(file_id))?;

    Some(expr_ty.render(db, None))
}

pub(crate) fn call_expr_type_info(
    db: &RootDatabase,
    FilePosition { file_id, offset }: FilePosition,
) -> Option<String> {
    let sema = Semantics::new(db, file_id);

    let file = sema.parse(file_id);

    let expr = algo::find_node_at_offset::<ast::AnyCallExpr>(file.syntax(), offset)?;
    let callable_ty: Ty = sema.get_call_expr_type(&expr.in_file(file_id))?.into();

    Some(callable_ty.render(db, None))
}
