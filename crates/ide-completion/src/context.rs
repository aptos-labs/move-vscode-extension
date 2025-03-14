use crate::completions::item_list::ItemListKind;
use crate::config::CompletionConfig;
use ide_db::RootDatabase;
use lang::files::FilePosition;
use lang::Semantics;
use syntax::algo::find_node_at_offset;
use syntax::SyntaxKind::*;
use syntax::{ast, AstNode, SyntaxToken, TextRange};

const COMPLETION_MARKER: &str = "raCompletionMarker";

/// The identifier we are currently completing.
#[derive(Debug)]
pub(crate) enum CompletionAnalysis {
    Item(ItemListKind),
    Path(ast::Path),
}

/// `CompletionContext` is created early during completion to figure out, where
/// exactly is the cursor, syntax-wise.
#[derive(Debug)]
pub(crate) struct CompletionContext<'a> {
    pub(crate) db: &'a RootDatabase,
    pub(crate) config: &'a CompletionConfig,
    pub(crate) position: FilePosition,

    /// The token before the cursor, in the original file.
    pub(crate) original_token: SyntaxToken,
}

impl CompletionContext<'_> {
    /// The range of the identifier that is being completed.
    pub(crate) fn source_range(&self) -> TextRange {
        let kind = self.original_token.kind();
        match kind {
            UNDERSCORE | INT_NUMBER => self.original_token.text_range(),
            // We want to consider all keywords in all editions.
            _ if kind.is_any_identifier() => self.original_token.text_range(),
            _ => TextRange::empty(self.position.offset),
        }
    }
}

impl<'a> CompletionContext<'a> {
    pub(crate) fn new(
        db: &'a RootDatabase,
        position @ FilePosition { file_id, offset }: FilePosition,
        config: &'a CompletionConfig,
    ) -> Option<(CompletionContext<'a>, CompletionAnalysis)> {
        let _p = tracing::info_span!("CompletionContext::new").entered();
        let sema = Semantics::new(db);

        let source_file = sema.parse(file_id);
        // always pick the token to the immediate left of the cursor, as that is what we are actually
        // completing on
        let original_token = source_file.syntax().token_at_offset(offset).left_biased()?;

        // todo: insert fake ident to fix the tree

        let ctx = CompletionContext {
            db,
            config,
            position,
            original_token,
        };

        if let Some(path) = find_node_at_offset::<ast::Path>(&source_file.syntax(), offset) {
            let analysis = CompletionAnalysis::Path(path);
            return Some((ctx, analysis));
        }

        let mut ident_parent = ctx.original_token.parent().unwrap();
        if ident_parent.kind().is_error() {
            ident_parent = ident_parent.parent().unwrap();
        }

        let item_list_kind = match ident_parent.kind() {
            ITEM_LIST => ItemListKind::Module,
            SOURCE_FILE => ItemListKind::SourceFile,
            _ => {
                // not an item list
                return None;
            }
        };
        let analysis = CompletionAnalysis::Item(item_list_kind);

        Some((ctx, analysis))
    }
}
