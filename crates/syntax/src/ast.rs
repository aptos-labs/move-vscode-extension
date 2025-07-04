// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

//! Abstract Syntax Tree, layered on top of untyped `SyntaxNode`s

mod generated;
mod traits;

pub mod edit;
pub mod idents;
pub mod make;
pub mod node_ext;
pub mod operators;
pub mod syntax_factory;
pub mod token_ext;
pub mod visibility;

use std::marker::PhantomData;

pub use self::{
    generated::{nodes::*, tokens::*},
    // expr_ext::{ArrayExprKind, BlockModifier, CallableExpr, ElseBranch, LiteralKind},
    node_ext::literal::LiteralKind,
    node_ext::struct_lit_field::StructLitFieldKind,
    node_ext::struct_pat_field::PatFieldKind,
    operators::{ArithOp, BinaryOp, CmpOp, LogicOp, Ordering, RangeOp, UnaryOp},
    token_ext::{CommentKind, CommentPlacement, CommentShape, IsString, QuoteOffsets},
    traits::{HasAttrs, HasItems, HasStmts, HasUseStmts, HoverDocsOwner, MslOnly},
    visibility::HasVisibility,
};
use crate::SyntaxKind::{CONST, ERROR};
use crate::{
    SyntaxKind,
    syntax_node::{SyntaxNode, SyntaxNodeChildren, SyntaxToken},
};

/// The main trait to go from untyped `SyntaxNode`  to a typed ast. The
/// conversion itself has zero runtime cost: ast and syntax nodes have exactly
/// the same representation: a pointer to the tree root and a pointer to the
/// node itself.
pub trait AstNode: std::fmt::Debug + Clone {
    /// This panics if the `SyntaxKind` is not statically known.
    fn kind() -> SyntaxKind
    where
        Self: Sized,
    {
        panic!("dynamic `SyntaxKind` for `AstNode::kind()`")
    }

    fn can_cast(kind: SyntaxKind) -> bool
    where
        Self: Sized;

    fn cast(syntax: SyntaxNode) -> Option<Self>
    where
        Self: Sized;

    fn syntax(&self) -> &SyntaxNode;
    fn clone_for_update(&self) -> Self
    where
        Self: Sized,
    {
        Self::cast(self.syntax().clone_for_update()).unwrap()
    }
    fn clone_subtree(&self) -> Self
    where
        Self: Sized,
    {
        Self::cast(self.syntax().clone_subtree()).unwrap()
    }
}

/// Like `AstNode`, but wraps tokens rather than interior nodes.
pub trait AstToken {
    fn can_cast(token: SyntaxKind) -> bool
    where
        Self: Sized;

    fn cast(syntax: SyntaxToken) -> Option<Self>
    where
        Self: Sized;

    fn syntax(&self) -> &SyntaxToken;

    fn text(&self) -> &str {
        self.syntax().text()
    }
}

/// An iterator over `SyntaxNode` children of a particular AST type.
#[derive(Debug, Clone)]
pub struct AstChildren<N> {
    inner: SyntaxNodeChildren,
    ph: PhantomData<N>,
}

impl<N> AstChildren<N> {
    fn new(parent: &SyntaxNode) -> Self {
        AstChildren {
            inner: parent.children(),
            ph: PhantomData,
        }
    }
}

impl<N: AstNode> Iterator for AstChildren<N> {
    type Item = N;
    fn next(&mut self) -> Option<N> {
        self.inner.find_map(N::cast)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AstError {
    pub(crate) syntax: SyntaxNode,
}
impl AstNode for AstError {
    #[inline]
    fn kind() -> SyntaxKind
    where
        Self: Sized,
    {
        ERROR
    }
    #[inline]
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == ERROR
    }
    #[inline]
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    #[inline]
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
}

mod support {
    use super::{AstChildren, AstNode, SyntaxKind, SyntaxNode, SyntaxToken};

    pub(super) fn child<N: AstNode>(parent: &SyntaxNode) -> Option<N> {
        parent.children().find_map(N::cast)
    }

    pub(super) fn children<N: AstNode>(parent: &SyntaxNode) -> AstChildren<N> {
        AstChildren::new(parent)
    }

    pub(super) fn token(parent: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxToken> {
        parent
            .children_with_tokens()
            .filter_map(|it| it.into_token())
            .find(|it| it.kind() == kind)
    }
}
