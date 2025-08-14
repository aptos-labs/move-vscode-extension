// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::{AstNode, SyntaxKind, ast};

impl ast::Path {
    pub fn path_address(&self) -> Option<ast::PathAddress> {
        self.segment()?.path_address()
    }

    pub fn reference(&self) -> ast::ReferenceElement {
        self.clone().into()
    }

    pub fn reference_name(&self) -> Option<String> {
        self.segment()?.name_ref().map(|it| it.as_string())
    }

    pub fn type_args(&self) -> Vec<ast::TypeArg> {
        self.segment()
            .and_then(|it| it.type_arg_list())
            .map(|it| it.type_arguments().collect())
            .unwrap_or_default()
    }

    pub fn path_expr(&self) -> Option<ast::PathExpr> {
        self.root_parent_of_type::<ast::PathExpr>()
    }

    pub fn root_parent_of_type<T: AstNode>(&self) -> Option<T> {
        self.root_path().syntax.parent_of_type::<T>()
    }

    pub fn root_parent_kind(&self) -> Option<SyntaxKind> {
        self.root_path().syntax().parent().map(|it| it.kind())
    }

    pub fn is_local(&self) -> bool {
        self.root_path() == self.base_path()
    }

    /** For `Foo::bar::baz::quux` path returns `Foo` */
    pub fn base_path(&self) -> ast::Path {
        let qualifier = self.qualifier();
        if let Some(qualifier) = qualifier {
            qualifier.base_path()
        } else {
            self.clone()
        }
    }

    /// for `Foo::bar` in `Foo::bar::baz::quux` returns `Foo::bar::baz::quux`
    pub fn root_path(&self) -> ast::Path {
        let parent_path = self.syntax.parent_of_type::<ast::Path>();
        if parent_path.is_some() {
            parent_path.unwrap().root_path()
        } else {
            self.clone()
        }
    }

    pub fn ident_token(&self) -> Option<ast::SyntaxToken> {
        self.segment()?.name_ref().and_then(|it| it.ident_token())
    }

    pub fn segments(&self) -> Vec<ast::PathSegment> {
        let mut segments = vec![];
        let mut current_path = Some(self.base_path());
        while let Some(path) = current_path {
            if let Some(segment) = path.segment() {
                segments.push(segment);
            }
            current_path = path.syntax.parent_of_type::<ast::Path>();
        }
        segments
    }
}
