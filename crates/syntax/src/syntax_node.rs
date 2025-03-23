//! This module defines Concrete Syntax Tree (CST), used by rust-analyzer.
//!
//! The CST includes comments and whitespace, provides a single node type,
//! `SyntaxNode`, and a basic traversal API (parent, children, siblings).
//!
//! The *real* implementation is in the (language-agnostic) `rowan` crate, this
//! module just wraps its API.

use rowan::{GreenNodeBuilder, Language};

use crate::{AstNode, Parse, SyntaxError, SyntaxKind, TextSize};

pub(crate) use rowan::GreenNode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Aptos {}
impl Language for Aptos {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> SyntaxKind {
        SyntaxKind::from(raw.0)
    }

    fn kind_to_raw(kind: SyntaxKind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind.into())
    }
}

pub type SyntaxNode = rowan::SyntaxNode<Aptos>;
pub type SyntaxToken = rowan::SyntaxToken<Aptos>;
pub type SyntaxElement = rowan::SyntaxElement<Aptos>;
pub type SyntaxNodeChildren = rowan::SyntaxNodeChildren<Aptos>;
pub type SyntaxElementChildren = rowan::SyntaxElementChildren<Aptos>;
pub type PreorderWithTokens = rowan::api::PreorderWithTokens<Aptos>;
pub type SyntaxNodeOrToken = rowan::NodeOrToken<SyntaxNode, SyntaxToken>;

pub trait IntoNodeOrToken {
    fn node_or_token(&self) -> SyntaxNodeOrToken;
}

impl<T: AstNode> IntoNodeOrToken for T {
    fn node_or_token(&self) -> SyntaxNodeOrToken {
        self.syntax().clone().into()
    }
}

#[derive(Default)]
pub struct SyntaxTreeBuilder {
    errors: Vec<SyntaxError>,
    inner: GreenNodeBuilder<'static>,
}

impl SyntaxTreeBuilder {
    pub(crate) fn finish_raw(self) -> (GreenNode, Vec<SyntaxError>) {
        let green = self.inner.finish();
        (green, self.errors)
    }

    pub fn finish(self) -> Parse {
        let (green, errors) = self.finish_raw();
        // Disable block validation, see https://github.com/rust-analyzer/rust-analyzer/pull/10357
        // if cfg!(debug_assertions) && false {
        //     let node = SyntaxNode::new_root(green.clone());
        //     crate::validation::validate_block_structure(&node);
        // }
        Parse::new(green, errors)
    }

    pub fn token(&mut self, kind: SyntaxKind, text: &str) {
        let kind = Aptos::kind_to_raw(kind);
        self.inner.token(kind, text);
    }

    pub fn start_node(&mut self, kind: SyntaxKind) {
        let kind = Aptos::kind_to_raw(kind);
        self.inner.start_node(kind);
    }

    pub fn finish_node(&mut self) {
        self.inner.finish_node();
    }

    pub fn error(&mut self, error: parser::ParseError, text_pos: TextSize) {
        self.errors.push(SyntaxError::new_at_offset(*error.0, text_pos));
    }
}
