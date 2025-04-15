use crate::completions::item_list::ItemListKind;
use crate::config::CompletionConfig;
use base_db::SourceDatabase;
use ide_db::RootDatabase;
use lang::Semantics;
use syntax::SyntaxKind::*;
use syntax::algo::find_node_at_offset;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxNodeExt;
use syntax::ast::node_ext::syntax_node::SyntaxElementExt;
use syntax::files::{FilePosition, InFile, InFileExt};
use syntax::{AstNode, SyntaxToken, TextRange, TextSize, ast};

const COMPLETION_MARKER: &str = "raCompletionMarker";

/// The identifier we are currently completing.
#[derive(Debug)]
pub(crate) enum CompletionAnalysis {
    Item(ItemListKind),
    Reference(ReferenceKind),
}

#[derive(Debug)]
pub enum ReferenceKind {
    Path(InFile<ast::Path>),
    FieldRef { receiver_expr: InFile<ast::Expr> },
}

/// `CompletionContext` is created early during completion to figure out, where
/// exactly is the cursor, syntax-wise.
#[derive(Debug)]
pub(crate) struct CompletionContext<'a> {
    pub(crate) db: &'a RootDatabase,
    pub(crate) config: &'a CompletionConfig,
    pub(crate) position: FilePosition,
    pub(crate) msl: bool,

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

    pub(crate) fn containing_module(&self) -> Option<ast::Module> {
        self.original_token.parent()?.containing_module()
    }
}

impl<'a> CompletionContext<'a> {
    pub(crate) fn new(
        db: &'a RootDatabase,
        position @ FilePosition { file_id, offset }: FilePosition,
        config: &'a CompletionConfig,
    ) -> Option<(CompletionContext<'a>, CompletionAnalysis)> {
        let _p = tracing::info_span!("CompletionContext::new").entered();
        let sema = Semantics::new(db, file_id);

        let original_file = sema.parse(file_id);
        // always pick the token to the immediate left of the cursor, as that is what we are actually
        // completing on
        let original_token = original_file.syntax().token_at_offset(offset).left_biased()?;

        // todo: insert fake ident to fix the tree
        // Insert a fake ident to get a valid parse tree. We will use this file
        // to determine context, though the original_file will be used for
        // actual completion.
        let fake_file = {
            let parse = db.parse(file_id);
            parse.reparse(TextRange::empty(offset), COMPLETION_MARKER).tree()
        };
        let fake_offset = offset + TextSize::of(COMPLETION_MARKER);

        let ctx = CompletionContext {
            db,
            config,
            position,
            msl: false,
            original_token,
        };

        if let Some(fake_ref) =
            find_node_at_offset::<ast::AnyReferenceElement>(&fake_file.syntax(), fake_offset)
        {
            let reference_kind = match fake_ref.syntax().kind() {
                PATH => {
                    let original_path =
                        find_node_at_offset::<ast::Path>(&original_file.syntax(), offset)?;
                    Some(ReferenceKind::Path(original_path.in_file(file_id)))
                }
                FIELD_REF => {
                    let original_receiver_expr =
                        find_node_at_offset::<ast::DotExpr>(&original_file.syntax(), offset)?
                            .receiver_expr();
                    Some(ReferenceKind::FieldRef {
                        receiver_expr: original_receiver_expr.in_file(file_id),
                    })
                }
                _ => None,
            };
            return reference_kind.and_then(|kind| {
                let analysis = CompletionAnalysis::Reference(kind);
                Some((ctx, analysis))
            });
        }

        let ident = ctx.original_token.clone();
        let mut ident_parent = ident.parent().unwrap();
        if ident_parent.kind().is_error() {
            ident_parent = ident_parent.parent().unwrap();
        }

        let ident_in_parent = ident_parent.child_or_token_at_range(ident.text_range()).unwrap();
        let ident_prev_sibling = ident_in_parent
            .prev_sibling_or_token_no_trivia()
            .map(|it| it.kind());

        let item_list_kind = match ident_parent.kind() {
            SOURCE_FILE => ItemListKind::SourceFile,
            MODULE => ItemListKind::Module,
            FUN if ident_prev_sibling == Some(VISIBILITY_MODIFIER) => {
                let fun = ident_parent.cast::<ast::Fun>().unwrap();
                ItemListKind::Function {
                    existing_modifiers: fun.modifiers_as_strings(),
                }
            }
            _ => {
                // not an item list
                return None;
            }
        };
        let analysis = CompletionAnalysis::Item(item_list_kind);

        Some((ctx, analysis))
    }
}
