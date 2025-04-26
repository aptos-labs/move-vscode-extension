use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use ide_db::Severity;
use lang::nameres::path_kind::{PathKind, QualifiedKind, path_kind};
use lang::types::ty::Ty;
use syntax::ast::ReferenceElement;
use syntax::ast::idents::PRIMITIVE_TYPES;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxNodeExt;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, ast};

pub(crate) fn unresolved_reference(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    reference: InFile<impl AstNode>,
) -> Option<()> {
    // for now
    let msl = reference.value.syntax().is_msl_context();
    if msl {
        return None;
    }
    if let Some(path) = reference.clone().cast_into::<ast::Path>() {
        unresolved_path(acc, ctx, path)?;
    }
    if let Some(method_or_dot) = reference.clone().cast_into::<ast::MethodOrDotExpr>() {
        unresolved_method_or_dot_expr(acc, ctx, method_or_dot)?;
    }
    if let Some(struct_lit_field) = reference.clone().cast_into::<ast::StructLitField>() {
        try_check_resolve(acc, ctx, struct_lit_field.map_into());
    }
    if let Some(struct_pat_field) = reference.clone().cast_into::<ast::StructPatField>() {
        try_check_resolve(acc, ctx, struct_pat_field.map_into());
    }

    Some(())
}

fn unresolved_path(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    reference: InFile<ast::Path>,
) -> Option<()> {
    let (_, path) = reference.clone().unpack();
    let path_name = path.reference_name()?;

    if let Some(_) = path.syntax().parent_of_type::<ast::PathType>() {
        if PRIMITIVE_TYPES.contains(&path_name.as_str()) {
            return None;
        }
    }

    // if msl && path_name.to_string().starts_with("result") {
    //     return None;
    // }

    if path.syntax().ancestor_of_type::<ast::AttrItem>(true).is_some() {
        return None;
    }

    let pkind = path_kind(path.clone(), false);
    match pkind {
        PathKind::Unknown
        | PathKind::NamedAddress(_)
        | PathKind::NamedAddressOrUnqualifiedPath { .. }
        | PathKind::ValueAddress(_) => (),
        PathKind::Unqualified { .. } => {
            try_check_resolve(acc, ctx, reference.map_into());
        }
        PathKind::Qualified { qualifier, kind, .. } => {
            match kind {
                QualifiedKind::ModuleItemOrEnumVariant
                | QualifiedKind::FQModuleItem
                | QualifiedKind::UseGroupItem => {
                    let resolved = ctx.sema.resolve(qualifier.into());
                    // qualifier is unresolved, no need to resolve current path
                    if resolved.is_none() {
                        return None;
                    }
                }
                _ => {}
            };
            try_check_resolve(acc, ctx, reference.map_into());
        }
    }
    Some(())
}

fn unresolved_method_or_dot_expr(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    reference: InFile<ast::MethodOrDotExpr>,
) -> Option<()> {
    let msl = reference.value.syntax().is_msl_context();
    let receiver_expr = reference.map_ref(|it| it.receiver_expr());
    let receiver_ty = ctx.sema.get_expr_type(&receiver_expr, msl)?.unwrap_all_refs();
    if matches!(receiver_ty, Ty::Unknown) {
        // no error if receiver item is unknown (won't proceed if unknown is nested)
        return None;
    }
    let (file_id, reference) = reference.unpack();
    match reference {
        ast::MethodOrDotExpr::MethodCallExpr(method_call_expr) => {
            let method_ref = method_call_expr.reference().in_file(file_id);
            try_check_resolve(acc, ctx, method_ref);
        }
        ast::MethodOrDotExpr::DotExpr(dot_expr) => {
            let dot_ref = dot_expr.reference().in_file(file_id);
            try_check_resolve(acc, ctx, dot_ref);
        }
    }
    Some(())
}

fn try_check_resolve(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    reference: InFile<ast::AnyReferenceElement>,
) -> Option<()> {
    let opt_entry = ctx.sema.resolve_in_file(reference.clone().map_into());
    if opt_entry.is_none() {
        let reference_node = reference.and_then_ref(|it| it.reference_node())?;
        let reference_name = reference.value.reference_name()?;
        acc.push(Diagnostic::new(
            DiagnosticCode::Lsp("unresolved-reference", Severity::Error),
            format!("Unresolved reference `{}`", reference_name),
            reference_node.file_range(),
        ));
    }
    Some(())
}
