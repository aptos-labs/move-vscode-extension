// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast::NamedElement;
use crate::ast::node_ext::syntax_element::SyntaxElementExt;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::{AstNode, ast};

impl ast::IdentPat {
    pub fn reference(&self) -> ast::ReferenceElement {
        self.clone().into()
    }

    pub fn ident_owner(&self) -> Option<ast::IdentPatOwner> {
        self.syntax().ancestor_strict::<ast::IdentPatOwner>()
    }
}
