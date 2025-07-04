// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

//! This file is actually hand-written, but the submodules are indeed generated.
#[rustfmt::skip]
pub(crate) mod nodes;
#[rustfmt::skip]
pub(crate) mod tokens;

use crate::{
    AstNode,
    SyntaxKind::{self, *},
    SyntaxNode,
};

pub(crate) use nodes::*;
