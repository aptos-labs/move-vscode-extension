use crate::loc::SyntaxLocFileExt;
use crate::nameres::namespaces::Ns;
use crate::nameres::scope::ScopeEntry;
use parser::SyntaxKind;
use parser::SyntaxKind::*;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxNodeExt;
use syntax::files::{InFile, InFileExt};
use syntax::{ast, AstNode};

#[tracing::instrument(level = "debug", skip(label))]
pub(crate) fn get_loop_labels_resolve_variants(label: InFile<ast::Label>) -> Vec<ScopeEntry> {
    let (file_id, label) = label.unpack();
    let mut entries = vec![];
    for scope in label.syntax().ancestors() {
        if is_label_barrier(scope.kind()) {
            break;
        }
        let opt_label_decl = scope.cast::<ast::LoopLike>().and_then(|it| it.label_decl());
        if let Some(label_decl) = opt_label_decl {
            let entry = label_decl_to_entry(label_decl.in_file(file_id));
            entries.push(entry);
        }
    }
    tracing::debug!(?entries);
    entries
}

fn label_decl_to_entry(label_decl: InFile<ast::LabelDecl>) -> ScopeEntry {
    let item_loc = label_decl.loc();
    // anything works here
    let item_ns = Ns::NAME;
    let entry = ScopeEntry {
        name: label_decl.value.name_as_string(),
        node_loc: item_loc,
        ns: item_ns,
        scope_adjustment: None,
    };
    entry
}

fn is_label_barrier(kind: SyntaxKind) -> bool {
    matches!(kind, LAMBDA_EXPR | FUN | SPEC_FUN | SPEC_INLINE_FUN | CONST)
}
