use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use base_db::SourceDatabase;
use ide_db::Severity;
use lang::nameres::path_kind::{PathKind, QualifiedKind, path_kind};
use lang::nameres::scope::{VecExt, into_field_shorthand_items};
use lang::node_ext::item_spec;
use lang::types::ty::Ty;
use syntax::ast::idents::PRIMITIVE_TYPES;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, ast};

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn find_unresolved_references(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    reference: InFile<ast::ReferenceElement>,
) -> Option<()> {
    if !ctx.config.unresolved_reference_enabled {
        return None;
    }
    if ctx.config.assists_only {
        // short-circuit
        return None;
    }
    let msl = reference.value.syntax().is_msl_context();
    if msl && is_special_msl_path(ctx.sema.db, reference.as_ref()).is_some() {
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

    // (_, _) = (1, 1);
    if path_name == "_" {
        for ancestor in path.syntax().ancestors() {
            if let Some((lhs, (_, op), _)) = ancestor.cast::<ast::BinExpr>().and_then(|it| it.unpack()) {
                // check for lhs of assignment expr
                if matches!(op, ast::BinaryOp::Assignment { .. })
                    && lhs.syntax().is_ancestor_of(&path.syntax())
                {
                    return None;
                }
            }
        }
    }

    if let Some(_) = path.syntax().parent_of_type::<ast::PathType>() {
        if PRIMITIVE_TYPES.contains(&path_name.as_str()) {
            return None;
        }
    }

    if path.syntax().ancestor_of_type::<ast::AttrItem>(true).is_some() {
        return None;
    }

    let pkind = path_kind(path.qualifier(), path.clone(), false)?;
    match pkind {
        PathKind::NamedAddress(_)
        | PathKind::NamedAddressOrUnqualifiedPath { .. }
        | PathKind::ValueAddress(_) => (),
        PathKind::FieldShorthand { .. } | PathKind::Unqualified { .. } => {
            try_check_resolve(acc, ctx, reference.map_into());
        }
        PathKind::Qualified { qualifier, kind, .. } => {
            match kind {
                QualifiedKind::ModuleItemOrEnumVariant
                | QualifiedKind::FQModuleItem
                | QualifiedKind::UseGroupItem => {
                    let resolved = ctx.sema.resolve(qualifier.into()).single_or_none();
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

fn is_special_msl_path(
    db: &dyn SourceDatabase,
    reference: InFile<&ast::ReferenceElement>,
) -> Option<()> {
    if reference
        .value
        .syntax()
        .has_ancestor_strict::<ast::ImplyIncludeExpr>()
    {
        return Some(());
    }

    let update_field_call_expr = reference.value.syntax().ancestors().find(|it| {
        it.cast::<ast::CallExpr>()
            .is_some_and(|call_expr| call_expr.syntax().text().to_string().starts_with("update_field"))
    });
    if update_field_call_expr.is_some() {
        return Some(());
    }

    let pragma_stmt = reference.value.syntax().ancestor_of_type::<ast::PragmaStmt>(true);
    if pragma_stmt.is_some() {
        return Some(());
    }

    let (file_id, reference) = reference.unpack();

    let path_expr = reference.clone().path()?.path_expr()?.in_file(file_id);
    if item_spec::get_item_spec_function(db, path_expr.as_ref()).is_some()
        && path_expr.syntax_text().starts_with("result")
    {
        return Some(());
    }

    None
}

fn unresolved_method_or_dot_expr(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    method_or_dot_expr: InFile<ast::MethodOrDotExpr>,
) -> Option<()> {
    let receiver_expr = method_or_dot_expr.map_ref(|it| it.receiver_expr());
    let receiver_ty = ctx.sema.get_expr_type(&receiver_expr)?.unwrap_all_refs();
    if matches!(receiver_ty, Ty::Unknown) {
        // no error if receiver item is unknown (won't proceed if unknown is nested)
        return None;
    }
    let (file_id, method_or_dot_expr) = method_or_dot_expr.unpack();
    let reference: ast::ReferenceElement = match method_or_dot_expr {
        ast::MethodOrDotExpr::MethodCallExpr(method_call_expr) => method_call_expr.into(),
        ast::MethodOrDotExpr::DotExpr(dot_expr) => dot_expr.into(),
    };
    try_check_resolve(acc, ctx, reference.in_file(file_id));
    Some(())
}

fn try_check_resolve(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    reference: InFile<ast::ReferenceElement>,
) -> Option<()> {
    let entries = ctx.sema.resolve_in_file(reference.clone().map_into());
    let reference_node = reference.and_then_ref(|it| it.reference_node())?;
    let reference_name = reference.value.reference_name()?;
    match entries.len() {
        0 => {
            acc.push(Diagnostic::new(
                DiagnosticCode::Lsp("unresolved-reference", Severity::Error),
                format!("Unresolved reference `{}`: cannot resolve", reference_name),
                reference_node.file_range(),
            ));
        }
        1 => (),
        _ => {
            if into_field_shorthand_items(ctx.sema.db, entries).is_some() {
                return None;
            }
            acc.push(Diagnostic::new(
                DiagnosticCode::Lsp("unresolved-reference", Severity::Error),
                format!(
                    "Unresolved reference `{}`: resolved to multiple elements",
                    reference_name
                ),
                reference_node.file_range(),
            ))
        }
    }
    Some(())
}
