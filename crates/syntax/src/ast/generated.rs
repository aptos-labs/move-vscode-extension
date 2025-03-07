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
