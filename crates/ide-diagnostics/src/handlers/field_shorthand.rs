// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use ide_db::Severity;
use ide_db::assist_context::LocalAssists;
use syntax::ast::syntax_factory::SyntaxFactory;
use syntax::files::{FileRange, InFile};
use syntax::{AstNode, ast};

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn struct_lit_field_can_be_simplified(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    struct_lit_field: InFile<ast::StructLitField>,
) -> Option<()> {
    if let Some(ast::StructLitFieldKind::Full { name_ref, expr, .. }) =
        struct_lit_field.as_ref().value.field_kind()
    {
        if let Some(expr) = expr
            && name_ref.as_string() == expr.syntax().text().to_string()
        {
            let fix_file_range = struct_lit_field.file_range();
            acc.push(
                Diagnostic::new(
                    DiagnosticCode::Lsp("lit-field-init-shorthand", Severity::WeakWarning),
                    "Expression can be simplified",
                    fix_file_range,
                )
                .with_local_fixes(lit_field_fix(
                    ctx,
                    struct_lit_field.as_ref(),
                    fix_file_range,
                )),
            )
        }
    }
    Some(())
}

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn struct_pat_field_can_be_simplified(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    struct_pat_field: InFile<ast::StructPatField>,
) -> Option<()> {
    let pat_field_kind = struct_pat_field.as_ref().value.field_kind();
    if let ast::PatFieldKind::Full {
        name_ref,
        pat: Some(ast::Pat::IdentPat(ident_pat)),
        ..
    } = pat_field_kind
        && name_ref.as_string() == ident_pat.syntax().text().to_string()
    {
        let fix_file_range = struct_pat_field.file_range();
        acc.push(
            Diagnostic::new(
                DiagnosticCode::Lsp("pat-field-init-shorthand", Severity::WeakWarning),
                "Expression can be simplified",
                fix_file_range,
            )
            .with_local_fixes(pat_field_fix(ctx, struct_pat_field.as_ref(), fix_file_range)),
        );
    }
    Some(())
}

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn schema_lit_field_can_be_simplified(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    schema_lit_field: InFile<ast::SchemaLitField>,
) -> Option<()> {
    if let Some(ast::SchemaLitFieldKind::Full { name_ref, expr, .. }) =
        schema_lit_field.as_ref().value.field_kind()
    {
        if let Some(expr) = expr
            && name_ref.as_string() == expr.syntax().text().to_string()
        {
            let fix_file_range = schema_lit_field.file_range();
            acc.push(
                Diagnostic::new(
                    DiagnosticCode::Lsp("schema-lit-field-init-shorthand", Severity::WeakWarning),
                    "Expression can be simplified",
                    fix_file_range,
                )
                .with_local_fixes(schema_lit_field_fix(
                    ctx,
                    schema_lit_field.as_ref(),
                    fix_file_range,
                )),
            )
        }
    }
    Some(())
}

fn lit_field_fix(
    ctx: &DiagnosticsContext<'_>,
    struct_lit_field: InFile<&ast::StructLitField>,
    diagnostic_range: FileRange,
) -> Option<LocalAssists> {
    let mut assists = ctx.local_assists_for_node(struct_lit_field)?;

    let lit_field = struct_lit_field.value;

    let struct_field_expr = lit_field.expr()?;
    assists.add_fix(
        "use-struct-lit-field-shorthand",
        "Use initialization shorthand",
        diagnostic_range.range,
        |editor| {
            let make = SyntaxFactory::new();
            let new_lit_field = make.lit_field_shorthand(struct_field_expr);

            editor.replace(lit_field.syntax(), new_lit_field.syntax());

            editor.add_mappings(make.finish_with_mappings());
        },
    );
    Some(assists)
}

fn pat_field_fix(
    ctx: &DiagnosticsContext<'_>,
    struct_pat_field: InFile<&ast::StructPatField>,
    diagnostic_range: FileRange,
) -> Option<LocalAssists> {
    let mut assists = ctx.local_assists_for_node(struct_pat_field)?;

    let pat_field = struct_pat_field.value;
    let field_ident_pat = pat_field.pat()?.ident_pat()?;

    assists.add_fix(
        "use-struct-pat-field-shorthand",
        "Use initialization shorthand",
        diagnostic_range.range,
        |editor| {
            let make = SyntaxFactory::new();
            let new_pat_field = make.pat_field_shorthand(field_ident_pat);

            editor.replace(pat_field.syntax(), new_pat_field.syntax());

            editor.add_mappings(make.finish_with_mappings());
        },
    );
    Some(assists)
}

#[tracing::instrument(level = "trace", skip_all)]
fn schema_lit_field_fix(
    ctx: &DiagnosticsContext<'_>,
    schema_lit_field: InFile<&ast::SchemaLitField>,
    diagnostic_range: FileRange,
) -> Option<LocalAssists> {
    let mut assists = ctx.local_assists_for_node(schema_lit_field)?;

    let lit_field = schema_lit_field.value;
    let struct_field_expr = lit_field.expr()?;
    assists.add_fix(
        "use-schema-lit-field-shorthand",
        "Use initialization shorthand",
        diagnostic_range.range,
        |edits| {
            let make = SyntaxFactory::new();
            let new_lit_field = make.schema_lit_field_shorthand(struct_field_expr);

            edits.replace(lit_field.syntax(), new_lit_field.syntax());

            edits.add_mappings(make.finish_with_mappings());
        },
    );
    Some(assists)
}
