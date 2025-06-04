use crate::completions::item_list::ItemListKind;
use crate::context::{COMPLETION_MARKER, CompletionAnalysis, ReferenceKind};
use syntax::SyntaxKind::{DOT_EXPR, FUN, MODULE, PATH, SOURCE_FILE, VISIBILITY_MODIFIER};
use syntax::algo::find_node_at_offset;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::ast::node_ext::syntax_element::SyntaxElementExt;
use syntax::{AstNode, SyntaxNode, SyntaxToken, TextSize, ast};

pub(crate) fn analyze(
    original_file: SyntaxNode,
    speculative_file: SyntaxNode,
    offset: TextSize,
    original_token: &SyntaxToken,
) -> Option<CompletionAnalysis> {
    let fake_offset = offset + TextSize::of(COMPLETION_MARKER);

    if let Some(fake_ref) = find_node_at_offset::<ast::ReferenceElement>(&speculative_file, fake_offset)
    {
        let reference_kind = match fake_ref.syntax().kind() {
            PATH => {
                let original_path = find_node_at_offset::<ast::Path>(&original_file, offset)?;
                Some(ReferenceKind::Path(original_path))
            }
            DOT_EXPR => {
                let original_receiver_expr =
                    find_node_at_offset::<ast::DotExpr>(&original_file, offset)?.receiver_expr();
                Some(ReferenceKind::FieldRef {
                    receiver_expr: original_receiver_expr,
                })
            }
            _ => None,
        };
        return reference_kind.map(|kind| CompletionAnalysis::Reference(kind));
    }

    let ident = original_token.clone();
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

    Some(CompletionAnalysis::Item(item_list_kind))
}
