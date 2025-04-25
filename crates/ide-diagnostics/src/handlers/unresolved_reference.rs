use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use ide_db::Severity;
use syntax::ast::ReferenceElement;
use syntax::ast::idents::PRIMITIVE_TYPES;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::InFile;
use syntax::{AstNode, ast};

pub(crate) fn unresolved_reference(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    reference: InFile<ast::AnyReferenceElement>,
) -> Option<()> {
    if let Some(path) = reference.clone().cast_into::<ast::Path>() {
        let path = path.value;
        if let Some(_) = path.syntax().parent_of_type::<ast::PathType>() {
            let path_name = path.reference_name()?;
            if PRIMITIVE_TYPES.contains(&path_name.as_str()) {
                return None;
            }
        }
    }
    let opt_entry = ctx.sema.resolve_in_file(reference.clone());
    if opt_entry.is_none() {
        let reference_name = reference.value.clone().syntax().text();
        acc.push(Diagnostic::new(
            DiagnosticCode::Lsp("unresolved-reference", Severity::Error),
            format!("Unresolved reference `{}`", reference_name),
            reference.file_range(),
        ));
    }
    Some(())
}
