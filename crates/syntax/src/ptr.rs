//! In rust-analyzer, syntax trees are transient objects.
//!
//! That means that we create trees when we need them, and tear them down to
//! save memory. In this architecture, hanging on to a particular syntax node
//! for a long time is ill-advisable, as that keeps the whole tree resident.
//!
//! Instead, we provide a [`SyntaxNodePtr`] type, which stores information about
//! *location* of a particular syntax node in a tree. Its a small type which can
//! be cheaply stored, and which can be resolved to a real [`SyntaxNode`] when
//! necessary.

use std::hash::Hash;

use crate::{AstNode, SyntaxNode, syntax_node::Aptos};

/// A "pointer" to a [`SyntaxNode`], via location in the source code.
pub type SyntaxNodePtr = rowan::ast::SyntaxNodePtr<Aptos>;

#[test]
fn test_local_syntax_ptr() {
    use crate::{AstNode, SourceFile, ast};

    let file = SourceFile::parse("module 0x1::m { fun main() {} }").ok().unwrap();
    let fun = file.syntax().descendants().find_map(ast::Fun::cast).unwrap();
    let ptr = SyntaxNodePtr::new(fun.syntax());
    let fun_syntax = ptr.to_node(file.syntax());
    assert_eq!(fun.syntax(), &fun_syntax);
}
