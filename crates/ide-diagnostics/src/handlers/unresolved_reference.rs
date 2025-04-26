use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use ide_db::Severity;
use lang::nameres::path_kind::{PathKind, QualifiedKind, path_kind};
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
        unresolved_path(acc, ctx, path)?;
    }
    Some(())
}

fn unresolved_path(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    reference: InFile<ast::Path>,
) -> Option<()> {
    let (_, path) = reference.clone().unpack();
    if let Some(_) = path.syntax().parent_of_type::<ast::PathType>() {
        let path_name = path.reference_name()?;
        if PRIMITIVE_TYPES.contains(&path_name.as_str()) {
            return None;
        }
    }
    let path_kind = path_kind(path, false);
    match path_kind {
        PathKind::Unknown
        | PathKind::NamedAddress(_)
        | PathKind::NamedAddressOrUnqualifiedPath { .. }
        | PathKind::ValueAddress(_) => (),
        PathKind::Unqualified { .. } => try_check_resolve(acc, ctx, reference),
        PathKind::Qualified { qualifier, kind, .. } => {
            match kind {
                QualifiedKind::Module { .. } => (),
                _ => {
                    let resolved = ctx.sema.resolve(qualifier.into());
                    // qualifier is unresolved, no need to resolve current path
                    if resolved.is_none() {
                        return None;
                    }
                }
            };
            try_check_resolve(acc, ctx, reference);
        }
    }
    Some(())
}

fn try_check_resolve(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    reference: InFile<ast::Path>,
) {
    let opt_entry = ctx.sema.resolve_in_file(reference.clone().map_into());
    if opt_entry.is_none() {
        let segment = reference.map(|it| it.segment().unwrap());
        let reference_name = segment.value.syntax().text();
        acc.push(Diagnostic::new(
            DiagnosticCode::Lsp("unresolved-reference", Severity::Error),
            format!("Unresolved reference `{}`", reference_name),
            segment.file_range(),
        ));
    }
}
