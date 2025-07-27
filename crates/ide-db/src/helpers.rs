// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::RootDatabase;
use lang::Semantics;
use syntax::ast::HasItems;
use syntax::files::{InFile, InFileExt};
use syntax::{AstToken, SyntaxKind, SyntaxToken, TokenAtOffset, ast};
use vfs::FileId;

/// Picks the token with the highest rank returned by the passed in function.
pub fn pick_best_token(
    tokens: TokenAtOffset<SyntaxToken>,
    f: impl Fn(SyntaxKind) -> usize,
) -> Option<SyntaxToken> {
    tokens.max_by_key(move |t| f(t.kind()))
}

pub fn pick_token<T: AstToken>(mut tokens: TokenAtOffset<SyntaxToken>) -> Option<T> {
    tokens.find_map(T::cast)
}

pub fn visit_file_defs(
    sema: &Semantics<'_, RootDatabase>,
    file_id: FileId,
    cb: &mut dyn FnMut(ast::NamedElement) -> Option<()>,
) {
    let file = sema.parse(file_id);
    for module in file.all_modules() {
        cb(module.clone().into());
        let module_items = module.named_items();
        for module_item in module_items {
            cb(module_item);
        }
    }
}

pub fn visit_item_specs(
    sema: &Semantics<'_, RootDatabase>,
    file_id: FileId,
    cb: &mut dyn FnMut(InFile<ast::ItemSpec>) -> Option<()>,
) {
    let file = sema.parse(file_id);
    for module in file.all_modules() {
        let module = module.in_file(file_id);
        for item_spec in module.as_ref().flat_map(|it| it.item_specs()) {
            cb(item_spec);
        }
    }
    for module_spec in file.module_specs() {
        for item_spec in module_spec.item_specs() {
            cb(item_spec.in_file(file_id));
        }
    }
}
