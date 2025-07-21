// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

mod analysis;

use crate::completions::item_list::ItemListKind;
use crate::config::CompletionConfig;
use crate::context::analysis::{AnalysisResult, completion_analysis};
use crate::item::{CompletionItem, CompletionItemBuilder, CompletionItemKind};
use base_db::inputs::InternFileId;
use base_db::source_db;
use ide_db::RootDatabase;
use lang::Semantics;
use lang::types::ty::Ty;
use syntax::SyntaxKind::*;
use syntax::ast::NameLike;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::files::FilePosition;
use syntax::{AstNode, SourceFile, SyntaxToken, T, TextRange, TextSize, algo, ast};

const COMPLETION_MARKER: &str = "raCompletionMarker";

/// The identifier we are currently completing.
#[derive(Debug)]
pub(crate) enum CompletionAnalysis {
    Item(ItemListKind),
    Reference(ReferenceKind),
    TypeParam { generic_element: ast::GenericElement },
}

#[derive(Debug)]
pub enum ReferenceKind {
    Path {
        fake_path: ast::Path,
    },
    DotExpr {
        original_receiver_expr: ast::Expr,
    },
    Label {
        fake_label: ast::Label,
        source_range: TextRange,
    },
    ItemSpecRef {
        original_item_spec: ast::ItemSpec,
    },
    StructLitField {
        original_struct_lit: ast::StructLit,
    },
    StructPatField {
        original_struct_pat: ast::StructPat,
    },
}

/// `CompletionContext` is created early during completion to figure out, where
/// exactly is the cursor, syntax-wise.
#[derive(Debug)]
pub(crate) struct CompletionContext<'db> {
    pub(crate) sema: Semantics<'db, RootDatabase>,
    pub(crate) db: &'db RootDatabase,
    pub(crate) config: &'db CompletionConfig,
    pub(crate) position: FilePosition,
    pub(crate) msl: bool,

    /// The token before the cursor, in the original file.
    pub(crate) original_token: SyntaxToken,

    /// The expected name of what we are completing.
    /// This is usually the parameter name of the function argument we are completing.
    pub(crate) expected_name: Option<NameLike>,
    /// The expected type of what we are completing.
    pub(crate) expected_type: Option<Ty>,
}

impl CompletionContext<'_> {
    pub(crate) fn original_file(&self) -> Option<SourceFile> {
        algo::containing_file_for_token(self.original_token.clone())
    }

    pub(crate) fn original_offset(&self) -> TextSize {
        self.position.offset
    }

    /// The range of the identifier that is being completed.
    pub(crate) fn source_range(&self) -> TextRange {
        let kind = self.original_token.kind();
        if matches!(kind, UNDERSCORE | INT_NUMBER | IDENT | QUOTE_IDENT) || kind.is_keyword() {
            self.original_token.text_range()
        } else {
            TextRange::empty(self.position.offset)
        }
    }

    pub(crate) fn containing_module(&self) -> Option<ast::Module> {
        self.original_token.parent()?.containing_module()
    }

    pub(crate) fn new_item(
        &self,
        kind: CompletionItemKind,
        label: impl Into<String>,
    ) -> CompletionItemBuilder {
        CompletionItem::new(kind, self.source_range(), label.into())
    }

    pub(crate) fn new_snippet_item(
        &self,
        kind: CompletionItemKind,
        snippet: impl Into<String>,
    ) -> CompletionItem {
        let snippet = snippet.into();
        let label = snippet.replace("$0", "");
        let label = label.trim();
        let mut item = CompletionItem::new(kind, self.source_range(), label);
        item.insert_snippet(snippet);
        item.build(self.db)
    }

    pub(crate) fn new_snippet_keyword(&self, snippet: impl Into<String>) -> CompletionItem {
        self.new_snippet_item(CompletionItemKind::Keyword, snippet)
    }
}

impl<'a> CompletionContext<'a> {
    pub(crate) fn new_with_analysis(
        db: &'a RootDatabase,
        position @ FilePosition { file_id, offset }: FilePosition,
        config: &'a CompletionConfig,
    ) -> Option<(CompletionContext<'a>, CompletionAnalysis)> {
        let _p = tracing::info_span!("CompletionContext::new").entered();
        let sema = Semantics::new(db, file_id);

        let original_file = sema.parse(file_id);

        // Insert a fake ident to get a valid parse tree. We will use this file
        // to determine context, though the original_file will be used for
        // actual completion.
        let file_with_fake_ident = {
            let parse = source_db::parse(db, file_id.intern(db));
            parse.reparse(TextRange::empty(offset), COMPLETION_MARKER).tree()
        };

        // always pick the token to the immediate left of the cursor, as that is what we are actually
        // completing on
        let original_token = original_file.syntax().token_at_offset(offset).left_biased()?;

        // try to skip completions on path with invalid colons
        if original_token.kind() == T![:] {
            // return if no prev token before colon
            let prev_token = original_token.prev_token()?;
            // only has a single colon
            if prev_token.kind() != T![:] {
                return None;
            }
        }

        let AnalysisResult {
            analysis,
            expected: (expected_type, expected_name),
        } = completion_analysis(
            &sema,
            &original_file,
            file_with_fake_ident.syntax().clone(),
            offset,
            &original_token,
        )?;

        let msl = original_token.parent().is_some_and(|it| it.is_msl_context());
        let ctx = CompletionContext {
            sema,
            db,
            config,
            position,
            msl,
            original_token,
            expected_name,
            expected_type,
        };

        Some((ctx, analysis))
    }
}
