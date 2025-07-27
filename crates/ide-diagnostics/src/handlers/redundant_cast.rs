// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use ide_db::Severity;
use ide_db::assist_context::Assists;
use ide_db::assists::AssistId;
use ide_db::label::Label;
use lang::types::ty::integer::IntegerKind;
use syntax::files::{FileRange, InFile};
use syntax::{AstNode, TextRange, ast};

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn redundant_integer_cast(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    cast_expr: InFile<ast::CastExpr>,
) -> Option<()> {
    let is_msl = cast_expr.is_msl();
    if is_msl {
        return None;
    }
    let inner_expr = cast_expr.as_ref().map(|it| it.expr());
    let inner_expr_ty = ctx.sema.get_expr_type(&inner_expr)?;
    // can only cast integer types
    let inner_integer_kind = inner_expr_ty.into_ty_integer()?;
    // cannot be redundant cast for untyped integer
    if inner_integer_kind == IntegerKind::Integer {
        return None;
    }
    let cast_type = cast_expr.as_ref().and_then(|it| it.type_())?;
    let cast_integer_kind = ctx.sema.lower_type(cast_type, is_msl).into_ty_integer()?;
    if inner_integer_kind == cast_integer_kind {
        let diagnostic_range = FileRange {
            file_id: cast_expr.file_id,
            range: TextRange::new(
                cast_expr.value.as_token()?.text_range().start(),
                cast_expr.value.type_()?.syntax().text_range().end(),
            ),
        };
        acc.push(
            Diagnostic::new(
                DiagnosticCode::Lsp("redundant-cast", Severity::Hint),
                "No cast needed",
                diagnostic_range,
            )
            .with_unused(true)
            .with_fixes(fixes(ctx, cast_expr, diagnostic_range)),
        );
    }
    Some(())
}

fn fixes(
    ctx: &DiagnosticsContext<'_>,
    cast_expr: InFile<ast::CastExpr>,
    diagnostic_range: FileRange,
) -> Option<Assists> {
    let (file_id, cast_expr) = cast_expr.unpack();
    let cast_expr_parent = cast_expr.syntax().parent()?;
    let mut assists = Assists::new(file_id, ctx.resolve.clone());
    assists.add(
        AssistId::quick_fix("remove-redundant-cast"),
        Label::new("Remove redundant cast".to_string()),
        diagnostic_range.range,
        |builder| {
            let mut file_edits = builder.make_editor(&cast_expr_parent);
            let inner_cast_expr = cast_expr.expr();
            file_edits.replace(cast_expr.syntax(), inner_cast_expr.syntax());
            builder.add_file_edits(file_id, file_edits);
        },
    );
    Some(assists)
}
