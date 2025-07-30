// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::assists::{Assist, AssistId, AssistResolveStrategy};
use crate::label::Label;
use crate::source_change::SourceChangeBuilder;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::files::InFile;
use syntax::syntax_editor::SyntaxEditor;
use syntax::{AstNode, SyntaxNode, TextRange, ast};
use vfs::FileId;

// todo: use Assists from rust-analyzer if we ever need multi-file assists
pub struct LocalAssists {
    file_id: FileId,
    source_file: ast::SourceFile,
    assists: Vec<Assist>,
    resolve: AssistResolveStrategy,
}

impl LocalAssists {
    pub fn new(context_node: InFile<&SyntaxNode>, resolve: AssistResolveStrategy) -> Option<Self> {
        let (file_id, containing_file) = context_node.and_then(|it| it.containing_file())?.unpack();
        Some(LocalAssists {
            file_id,
            source_file: containing_file,
            assists: Vec::new(),
            resolve,
        })
    }

    pub fn assists(self) -> Vec<Assist> {
        self.assists
    }

    pub fn add_fix(
        &mut self,
        id: &'static str,
        label: impl Into<String>,
        target: TextRange,
        f: impl FnOnce(&mut SyntaxEditor),
    ) -> Option<()> {
        let id = AssistId::quick_fix(id);
        let label = label.into();
        let source_change = if self.resolve.should_resolve(&id) {
            let mut builder = SourceChangeBuilder::new(self.file_id);
            let mut editor = builder.make_editor(self.source_file.syntax());
            f(&mut editor);
            builder.add_file_edits(self.file_id, editor);
            Some(builder.finish())
        } else {
            None
        };
        self.assists.push(Assist {
            id,
            label: Label::new(label),
            target,
            source_change,
            command: None,
        });
        Some(())
    }
}
