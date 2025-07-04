// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast::node_ext::syntax_element::SyntaxElementExt;
use crate::syntax_editor::Element;
use crate::{SyntaxElement, SyntaxNode, SyntaxToken};

impl SyntaxElementExt for SyntaxToken {
    fn to_syntax_element(&self) -> SyntaxElement {
        self.syntax_element()
    }
}
