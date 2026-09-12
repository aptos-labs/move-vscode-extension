// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::nameres;
use base_db::SourceDatabase;
use syntax::ast::node_ext::syntax_element::SyntaxElementExt;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, SyntaxElement, ast};

pub trait ItemSpecExt {
    fn related_item(&self, db: &dyn SourceDatabase) -> Option<InFile<ast::ItemSpecItem>>;
}

impl ItemSpecExt for InFile<ast::ItemSpec> {
    fn related_item(&self, db: &dyn SourceDatabase) -> Option<InFile<ast::ItemSpecItem>> {
        let item_spec_ref = self.and_then_ref(|it| it.item_spec_ref())?;
        let entry = nameres::resolve(db, item_spec_ref)?;
        entry.cast_into(db)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AttachedSpecItem {
    Fun(InFile<ast::Fun>),
    StructOrEnum(InFile<ast::StructOrEnum>),
    UnresolvedItem,
    Module,
    Lambda(ast::LambdaExpr),
}

pub fn attached_spec_item(
    db: &dyn SourceDatabase,
    element: InFile<SyntaxElement>,
) -> Option<AttachedSpecItem> {
    let (file_id, element) = element.unpack();
    if let Some(spec_block_expr) = element.ancestor_strict::<ast::SpecBlockExpr>()
        && let Some(lambda_expr) = spec_block_expr.syntax().parent_of_type::<ast::LambdaExpr>()
    {
        return Some(AttachedSpecItem::Lambda(lambda_expr));
    }
    if let Some(item_spec) = element.containing_item_spec() {
        return match item_spec.item_spec_ref() {
            Some(item_spec_ref) => {
                let related_item = nameres::resolve(db, item_spec_ref.in_file(file_id))
                    .and_then(|it| it.cast_into::<ast::ItemSpecItem>(db));
                match related_item {
                    Some(InFile {
                        file_id,
                        value: ast::ItemSpecItem::StructOrEnum(struct_or_enum),
                    }) => Some(AttachedSpecItem::StructOrEnum(struct_or_enum.in_file(file_id))),
                    Some(InFile {
                        file_id,
                        value: ast::ItemSpecItem::Fun(fun),
                    }) => Some(AttachedSpecItem::Fun(fun.in_file(file_id))),
                    None => Some(AttachedSpecItem::UnresolvedItem),
                }
            }
            // spec module
            None => Some(AttachedSpecItem::Module),
        };
    }
    None
}
