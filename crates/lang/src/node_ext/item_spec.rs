// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::nameres;
use crate::types::lowering::TyLowering;
use crate::types::ty::Ty;
use crate::types::ty::adt::TyAdt;
use base_db::SourceDatabase;
use regex::Regex;
use std::sync::LazyLock;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::files::InFile;
use syntax::{AstNode, ast};

pub trait ItemSpecExt {
    fn item(&self, db: &dyn SourceDatabase) -> Option<InFile<ast::ItemSpecItem>>;
}

impl ItemSpecExt for InFile<ast::ItemSpec> {
    fn item(&self, db: &dyn SourceDatabase) -> Option<InFile<ast::ItemSpecItem>> {
        let item_spec_ref = self.and_then_ref(|it| it.item_spec_ref())?;
        let entry = nameres::resolve(db, item_spec_ref)?;
        entry.cast_into(db)
    }
}

pub fn infer_special_path_expr_for_item_spec(
    db: &dyn SourceDatabase,
    path_expr: InFile<&ast::PathExpr>,
) -> Option<Ty> {
    let path_name = path_expr.value.path().reference_name()?;
    // short-circuit
    if !path_name.starts_with("result") && path_name != "self" {
        return None;
    }
    let item_spec_item = path_expr
        .and_then(|it| it.syntax().containing_item_spec())?
        .item(db)?;
    if path_name.starts_with("result")
        && let Some(fun) = item_spec_item.clone().cast_into::<ast::Fun>()
    {
        let ty_lowering = TyLowering::new(db, true);
        let fun_return_type = fun
            .and_then_ref(|it| it.return_type())
            .map(|it| ty_lowering.lower_type(it))
            .unwrap_or(Ty::Unit);
        if path_name == "result" {
            return Some(fun_return_type);
        }
        let (_, [index]) = TUPLE_RESULT_REGEX.captures(&path_name)?.extract();
        let tuple_index = index.parse::<usize>().unwrap();
        let member_ty = fun_return_type
            .into_ty_tuple()
            .and_then(|ty_tuple| ty_tuple.types.get(tuple_index - 1).cloned());

        return Some(member_ty.unwrap_or(Ty::Unknown));
    }
    if path_name == "self"
        && let Some(struct_or_enum) = item_spec_item.cast_into::<ast::StructOrEnum>()
    {
        return Some(TyAdt::new(struct_or_enum).into());
    }
    None
}

static TUPLE_RESULT_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^result_([1-9])$").unwrap());
