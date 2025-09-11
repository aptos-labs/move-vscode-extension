// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use base_db::inputs::InternFileId;
use base_db::{SourceDatabase, source_db};
use std::fmt::Formatter;
use std::{env, fmt};
use syntax::files::{FileRange, InFile};
use syntax::{AstNode, SourceFile, SyntaxNode, TextRange, TextSize};
use syntax::{SyntaxKind, SyntaxKind::*, SyntaxNodePtr};
use vfs::FileId;

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct SyntaxLoc {
    file_id: FileId,
    syntax_ptr: SyntaxNodePtr,
    // only for debugging here, might be removed in the future
    node_name: Option<String>,
}

impl SyntaxLoc {
    pub fn from_ast_node(file_id: FileId, ast_node: &impl AstNode) -> Self {
        Self::from_syntax_node(file_id, ast_node.syntax())
    }

    pub fn from_file_syntax_node(syntax_node: &InFile<SyntaxNode>) -> Self {
        let n = syntax_node.as_ref();
        Self::from_syntax_node(n.file_id, n.value)
    }

    pub fn from_syntax_node(file_id: FileId, syntax_node: &SyntaxNode) -> Self {
        let mut node_name: Option<String> = None;
        if env::var("APT_SYNTAXLOC_DEBUG").is_ok() {
            let _p = tracing::debug_span!("SyntaxLoc::from_ast_node::node_name").entered();
            node_name = syntax_node
                .children_with_tokens()
                .find(|child| {
                    let kind = child.kind();
                    kind == NAME || kind == NAME_REF || kind == PATH_SEGMENT || kind == QUOTE_IDENT
                })
                .map(|it| it.to_string());
        }

        SyntaxLoc {
            file_id: file_id.to_owned(),
            syntax_ptr: SyntaxNodePtr::new(syntax_node),
            node_name,
        }
    }

    pub fn to_ast<T: AstNode>(&self, db: &dyn SourceDatabase) -> Option<InFile<T>> {
        let file = self.get_source_file(db)?;
        self.syntax_ptr
            .try_to_node(file.syntax())
            .and_then(|node| T::cast(node))
            .map(|ast_node| InFile::new(self.file_id, ast_node))
    }

    pub fn to_syntax_node(&self, db: &dyn SourceDatabase) -> Option<InFile<SyntaxNode>> {
        let file = self.get_source_file(db)?;
        self.syntax_ptr
            .try_to_node(file.syntax())
            .map(|ast_node| InFile::new(self.file_id, ast_node))
    }

    pub fn file_id(&self) -> FileId {
        self.file_id
    }

    pub fn syntax_ptr(&self) -> SyntaxNodePtr {
        self.syntax_ptr
    }

    pub fn file_range(&self) -> FileRange {
        FileRange {
            file_id: self.file_id,
            range: self.syntax_ptr.text_range(),
        }
    }

    pub fn text_range(&self) -> TextRange {
        self.syntax_ptr.text_range()
    }

    pub fn kind(&self) -> SyntaxKind {
        self.syntax_ptr.kind()
    }

    pub fn node_offset(&self) -> TextSize {
        self.syntax_ptr.text_range().end()
    }

    pub fn node_name(&self) -> Option<String> {
        self.node_name.to_owned()
    }

    pub fn contains(&self, other_loc: &SyntaxLoc) -> bool {
        self.file_id == other_loc.file_id
            && self
                .syntax_ptr
                .text_range()
                .contains_range(other_loc.syntax_ptr.text_range())
    }

    fn get_source_file(&self, db: &dyn SourceDatabase) -> Option<SourceFile> {
        let file = source_db::parse(db, self.file_id.intern(db)).tree();
        if !file.syntax().text_range().contains_inclusive(self.node_offset()) {
            tracing::error!(
                "stale cache error: {:?} is outside of the file range {:?}",
                self,
                file.syntax().text_range()
            );
            return None;
        }
        Some(file)
    }
}

impl fmt::Debug for SyntaxLoc {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.node_name {
            Some(name) => f
                .debug_tuple("Loc")
                .field(&format!(
                    "{:?} named '{}' at {}::{:?}",
                    self.syntax_ptr.kind(),
                    name,
                    self.file_id.index(),
                    self.node_offset()
                ))
                .finish(),
            None => f
                .debug_tuple("Loc")
                .field(&format!(
                    "{:?} at {}::{:?}",
                    self.syntax_ptr.kind(),
                    self.file_id.index(),
                    self.node_offset()
                ))
                .finish(),
        }
    }
}

// for `revisions` parameter, see https://github.com/salsa-rs/salsa/pull/911
// #[salsa_macros::interned(debug, revisions = usize::MAX)]
#[salsa_macros::interned(debug)]
pub struct SyntaxLocInput {
    pub syntax_loc: SyntaxLoc,
}

impl SyntaxLocInput<'_> {
    pub fn to_ast<T: AstNode>(&self, db: &dyn SourceDatabase) -> Option<InFile<T>> {
        self.syntax_loc(db).to_ast(db)
    }
}

pub trait SyntaxLocFileExt {
    fn loc(&self) -> SyntaxLoc;
}

impl<T: AstNode> SyntaxLocFileExt for InFile<T> {
    fn loc(&self) -> SyntaxLoc {
        SyntaxLoc::from_ast_node(self.file_id, &self.value)
    }
}

pub trait SyntaxLocNodeExt {
    fn loc(&self, file_id: FileId) -> SyntaxLoc;
}

impl<T: AstNode> SyntaxLocNodeExt for T {
    fn loc(&self, file_id: FileId) -> SyntaxLoc {
        SyntaxLoc::from_ast_node(file_id, self)
    }
}
