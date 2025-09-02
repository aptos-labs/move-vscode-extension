// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use super::{SyntaxFactory, ast_from_text, expr_item_from_text, module_item_from_text};
use crate::ast::make::quote::quote;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::parse::SyntaxKind;
use crate::syntax_editor::mapping::SyntaxMappingBuilder;
use crate::{
    AstNode, SourceFile, SyntaxElement, SyntaxNode, SyntaxToken,
    ast::{self, make},
};
use stdx::itertools::Itertools;

impl SyntaxFactory {
    pub fn arg_list(&self, args: impl IntoIterator<Item = ast::Expr>) -> ast::ValueArgList {
        let (args, input) = iterator_input(args);
        let args = args.into_iter().format(", ");
        ast_from_text::<ast::ValueArgList>(&format!("module 0x1::m {{ fun main() {{ call({args}) }} }}"))
            .clone_for_update()
    }

    pub fn ident_pat(&self, ident_name: &str) -> ast::IdentPat {
        ast_from_text::<ast::IdentPat>(&format!(
            "module 0x1::m {{ fun main() {{ let {ident_name}; }} }}"
        ))
        .clone_for_update()
    }

    pub fn name(&self, name: &str) -> ast::Name {
        ast_from_text::<ast::Name>(&format!("module {name}")).clone_for_update()
    }

    pub fn name_ref(&self, name_ref: &str) -> ast::NameRef {
        quote! {
            NameRef {
                [IDENT format!("{name_ref}")]
            }
        }
        .clone_for_update()
    }

    pub fn attr(&self, attr_text: &str) -> ast::Attr {
        ast_from_text::<ast::Attr>(&format!("#[{attr_text}]module 0x1::m {{}}")).clone_for_update()
    }

    pub fn token(&self, kind: SyntaxKind) -> SyntaxToken {
        make::tokens::SOURCE_FILE
            .tree()
            .syntax()
            .clone_for_update()
            .descendants_with_tokens()
            .filter_map(|it| it.into_token())
            .find(|it| it.kind() == kind)
            .unwrap_or_else(|| panic!("unhandled token: {kind:?}"))
    }

    pub fn whitespace(&self, text: &str) -> SyntaxToken {
        assert!(text.trim().is_empty());
        let sf = SourceFile::parse(text).ok().unwrap();
        sf.syntax()
            .clone_for_update()
            .first_child_or_token()
            .unwrap()
            .into_token()
            .unwrap()
    }

    pub fn newline(&self) -> SyntaxToken {
        self.whitespace("\n")
    }
}

// We need to collect `input` here instead of taking `impl IntoIterator + Clone`,
// because if we took `impl IntoIterator + Clone`, that could be something like an
// `Iterator::map` with a closure that also makes use of a `SyntaxFactory` constructor.
//
// In that case, the iterator would be evaluated inside of the call to `map_children`,
// and the inner constructor would try to take a mutable borrow of the mappings `RefCell`,
// which would panic since it's already being mutably borrowed in the outer constructor.
pub(crate) fn iterator_input<N: AstNode>(
    input: impl IntoIterator<Item = N>,
) -> (Vec<N>, Vec<SyntaxNode>) {
    input
        .into_iter()
        .map(|it| {
            let syntax = it.syntax().clone();
            (it, syntax)
        })
        .collect()
}
