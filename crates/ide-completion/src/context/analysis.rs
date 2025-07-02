use crate::completions::item_list::ItemListKind;
use crate::context::{COMPLETION_MARKER, CompletionAnalysis, ReferenceKind};
use syntax::SyntaxKind::{FUN, MODULE, SOURCE_FILE, VISIBILITY_MODIFIER};
use syntax::algo::find_node_at_offset;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::ast::node_ext::syntax_element::SyntaxElementExt;
use syntax::{AstNode, SyntaxNode, SyntaxToken, TextRange, TextSize, ast};

pub(crate) fn analyze(
    original_file: SyntaxNode,
    fake_file: SyntaxNode,
    original_offset: TextSize,
    original_token: &SyntaxToken,
) -> Option<CompletionAnalysis> {
    // as we insert after the offset, right biased will *always* pick the identifier no matter
    // if there is an ident already typed or not
    let fake_token = fake_file.token_at_offset(original_offset).right_biased()?;

    if !original_token.kind().is_keyword() {
        if let Some(fake_ref) = fake_token
            .parent_ancestors()
            .find_map(ast::ReferenceElement::cast)
        {
            return analyze_ref(&fake_ref, original_file, original_offset);
        }
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
        MODULE => {
            let module = ident_parent.cast::<ast::Module>().unwrap();
            // no completions if module has no '{' yet
            let l_curly_token = module.l_curly_token()?;
            // if it's before the '{', then no completions available
            if ident.text_range().end() < l_curly_token.text_range().start() {
                return None;
            }
            ItemListKind::Module
        }
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

fn analyze_ref(
    ref_element: &ast::ReferenceElement,
    original_file: SyntaxNode,
    original_offset: TextSize,
) -> Option<CompletionAnalysis> {
    let reference_kind = match ref_element {
        ast::ReferenceElement::Path(fake_path) => {
            let original_path = find_node_at_offset::<ast::Path>(&original_file, original_offset);
            Some(ReferenceKind::Path {
                original_path,
                fake_path: fake_path.clone(),
            })
        }
        ast::ReferenceElement::DotExpr(_) => {
            let original_receiver_expr =
                find_node_at_offset::<ast::DotExpr>(&original_file, original_offset)?.receiver_expr();
            Some(ReferenceKind::FieldRef {
                receiver_expr: original_receiver_expr,
            })
        }
        ast::ReferenceElement::Label(fake_label) => {
            let fake_range = fake_label.syntax().text_range();
            Some(ReferenceKind::Label {
                fake_label: fake_label.clone(),
                source_range: TextRange::new(
                    fake_range.start(),
                    fake_range.end() - TextSize::of(COMPLETION_MARKER),
                ),
            })
        }
        _ => None,
    };
    reference_kind.map(|kind| CompletionAnalysis::Reference(kind))
}
