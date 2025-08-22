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

    pub fn path_segment_from_text(&self, name: impl Into<String>) -> ast::PathSegment {
        let name = name.into();
        ast_from_text::<ast::PathSegment>(&format!(
            "module 0x1::m {{ fun main() {{ let _ = {name}; }}}}"
        ))
    }

    pub fn path_segment_from_value_address(&self, value_address: impl Into<String>) -> ast::PathSegment {
        let value_address = value_address.into();
        let path = ast_from_text::<ast::Path>(&format!(
            "module 0x1::m {{ const MY_CONST: {value_address}::my_path = 1; }}"
        ));
        let qualifier = path.qualifier().unwrap();
        qualifier
            .path_address()
            .unwrap()
            .syntax()
            .parent_of_type()
            .unwrap()
    }

    pub fn path_segment(&self, name_ref: ast::NameRef) -> ast::PathSegment {
        let ast = ast_from_text::<ast::PathSegment>(&format!(
            "module 0x1::m {{ fun main() {{ let _ = {name_ref}; }}}}"
        ));

        if let Some(mut mapping) = self.mappings() {
            let mut builder = SyntaxMappingBuilder::new(ast.syntax().clone());
            builder.map_node(
                name_ref.syntax().clone(),
                ast.name_ref().unwrap().syntax().clone(),
            );
            builder.finish(&mut mapping);
        }

        ast
    }

    pub fn path_from_segments(&self, segments: impl IntoIterator<Item = ast::PathSegment>) -> ast::Path {
        let segments = segments.into_iter().map(|it| it.syntax().clone()).join("::");
        expr_item_from_text(&segments)
    }

    pub fn use_stmt(&self, path: ast::Path) -> ast::UseStmt {
        let path_text = path.syntax().text();
        module_item_from_text::<ast::UseStmt>(&format!("use {path_text};")).clone_for_update()
    }

    pub fn use_speck(&self, path: ast::Path, alias: Option<ast::UseAlias>) -> ast::UseSpeck {
        let mut buf = "use ".to_string();
        buf += &path.syntax().to_string();
        if let Some(alias) = alias {
            stdx::format_to!(buf, " {alias}");
        }
        module_item_from_text::<ast::UseSpeck>(&buf).clone_for_update()
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
