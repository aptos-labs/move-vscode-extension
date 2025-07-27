// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use ide_db::Severity;
use ide_db::assist_context::Assists;
use ide_db::assists::AssistId;
use ide_db::label::Label;
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
                .with_fixes(lit_field_fix(
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
            .with_fixes(pat_field_fix(ctx, struct_pat_field.as_ref(), fix_file_range)),
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
                .with_fixes(schema_lit_field_fix(
                    ctx,
                    schema_lit_field.as_ref(),
                    fix_file_range,
                )),
            )
        }
    }
    Some(())
}

#[tracing::instrument(level = "trace", skip_all)]
fn lit_field_fix(
    ctx: &DiagnosticsContext<'_>,
    struct_lit_field: InFile<&ast::StructLitField>,
    diagnostic_range: FileRange,
) -> Option<Assists> {
    let (file_id, lit_field) = struct_lit_field.unpack();
    let struct_field_expr = lit_field.expr()?;

    let mut assists = ctx.assists_for_file(file_id);
    assists.add(
        AssistId::quick_fix("use-struct-lit-field-shorthand"),
        Label::new("Use initialization shorthand".to_string()),
        diagnostic_range.range,
        |builder| {
            let make = SyntaxFactory::new();
            let new_lit_field = make.lit_field_shorthand(struct_field_expr);

            let mut edits = builder.make_editor(lit_field.struct_lit().syntax());
            edits.replace(lit_field.syntax(), new_lit_field.syntax());

            edits.add_mappings(make.finish_with_mappings());
            builder.add_file_edits(file_id, edits);
        },
    );
    Some(assists)
}

#[tracing::instrument(level = "trace", skip_all)]
fn pat_field_fix(
    ctx: &DiagnosticsContext<'_>,
    struct_pat_field: InFile<&ast::StructPatField>,
    diagnostic_range: FileRange,
) -> Option<Assists> {
    let (file_id, pat_field) = struct_pat_field.unpack();
    let field_ident_pat = pat_field.pat()?.ident_pat()?;

    let mut assists = ctx.assists_for_file(file_id);
    assists.add(
        AssistId::quick_fix("use-struct-pat-field-shorthand"),
        Label::new("Use initialization shorthand".to_string()),
        diagnostic_range.range,
        |builder| {
            let make = SyntaxFactory::new();
            let new_pat_field = make.pat_field_shorthand(field_ident_pat);

            let mut edits = builder.make_editor(pat_field.struct_pat().syntax());
            edits.replace(pat_field.syntax(), new_pat_field.syntax());

            edits.add_mappings(make.finish_with_mappings());
            builder.add_file_edits(file_id, edits);
        },
    );
    Some(assists)
}

#[tracing::instrument(level = "trace", skip_all)]
fn schema_lit_field_fix(
    ctx: &DiagnosticsContext<'_>,
    schema_lit_field: InFile<&ast::SchemaLitField>,
    diagnostic_range: FileRange,
) -> Option<Assists> {
    let (file_id, lit_field) = schema_lit_field.unpack();
    let struct_field_expr = lit_field.expr()?;
    let schema_lit = lit_field.schema_lit()?;

    let mut assists = ctx.assists_for_file(file_id);
    assists.add(
        AssistId::quick_fix("use-schema-lit-field-shorthand"),
        Label::new("Use initialization shorthand".to_string()),
        diagnostic_range.range,
        |builder| {
            let make = SyntaxFactory::new();
            let new_lit_field = make.schema_lit_field_shorthand(struct_field_expr);

            let mut edits = builder.make_editor(schema_lit.syntax());
            edits.replace(lit_field.syntax(), new_lit_field.syntax());

            edits.add_mappings(make.finish_with_mappings());
            builder.add_file_edits(file_id, edits);
        },
    );
    Some(assists)
}
