// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast::{AstChildren, UseStmt, support};
use crate::{AstNode, ast};

pub trait UseStmtsOwner: AstNode {
    #[inline]
    fn use_stmts(&self) -> AstChildren<UseStmt> {
        support::children(&self.syntax())
    }

    fn use_specks(&self) -> Vec<ast::UseSpeck> {
        self.use_stmts()
            .into_iter()
            .filter_map(|i| i.use_speck())
            .flat_map(|use_speck| {
                if let Some(use_group) = use_speck.use_group() {
                    let mut v = vec![use_speck];
                    v.extend(use_group.use_specks());
                    v
                } else {
                    vec![use_speck]
                }
            })
            .collect()
    }
}
