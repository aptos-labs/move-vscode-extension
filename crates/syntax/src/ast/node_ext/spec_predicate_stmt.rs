// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;

impl ast::SpecPredicateStmt {
    pub fn kind(&self) -> Option<SpecPredicateKind> {
        if self.assert_token().is_some() {
            return Some(SpecPredicateKind::Assert);
        }
        if self.assume_token().is_some() {
            return Some(SpecPredicateKind::Assume);
        }
        if self.requires_token().is_some() {
            return Some(SpecPredicateKind::Requires);
        }
        if self.ensures_token().is_some() {
            return Some(SpecPredicateKind::Ensures);
        }
        if self.decreases_token().is_some() {
            return Some(SpecPredicateKind::Decreases);
        }
        if self.modifies_token().is_some() {
            return Some(SpecPredicateKind::Modifies);
        }
        None
    }
}

pub enum SpecPredicateKind {
    Assert,
    Assume,
    Requires,
    Ensures,
    Decreases,
    Modifies,
}
