use parser::SyntaxKind;
use parser::SyntaxKind::*;
use syntax::{SyntaxNode, ast};

pub(crate) fn get_loop_labels_resolve_variants(label: ast::Label) {

}

fn is_label_barrier(kind: SyntaxKind) -> bool {
    matches!(kind, LAMBDA_EXPR | FUN | SPEC_FUN | SPEC_INLINE_FUN | CONST)
}
