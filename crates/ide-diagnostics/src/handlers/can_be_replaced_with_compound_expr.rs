// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

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
pub(crate) fn can_be_replaced_with_compound_expr(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    bin_expr: InFile<ast::BinExpr>,
) -> Option<()> {
    let (lhs_expr, (_, op_kind), rhs_expr) = bin_expr.value.unpack()?;
    let rhs_expr = rhs_expr?;
    if let ast::BinaryOp::Assignment { op: None } = op_kind {
        let rhs_bin_expr = rhs_expr.bin_expr()?;
        let (argument_expr, _, _) = rhs_bin_expr.clone().unpack()?;
        if lhs_expr.syntax().green() == argument_expr.syntax().green() {
            acc.push(
                Diagnostic::new(
                    DiagnosticCode::Lsp("replace-with-compound-expr", Severity::WeakWarning),
                    "Can be replaced with compound assignment",
                    bin_expr.file_range(),
                )
                .with_fixes(fixes(ctx, bin_expr.clone(), bin_expr.file_range())),
            );
        }
    }

    Some(())
}

fn fixes(
    ctx: &DiagnosticsContext<'_>,
    bin_expr: InFile<ast::BinExpr>,
    diagnostic_range: FileRange,
) -> Option<Assists> {
    let (file_id, bin_expr) = bin_expr.unpack();
    let (lhs_expr, _, rhs_expr) = bin_expr.clone().unpack()?;
    let initializer_expr = rhs_expr?.bin_expr()?;

    let (_, (_, rhs_op), rhs_expr) = initializer_expr.unpack()?;
    let rhs_expr = rhs_expr?;

    let mut assists = Assists::new(file_id, ctx.resolve.clone());
    if let ast::BinaryOp::ArithOp(arith_op) = rhs_op {
        let compound_op = ast::BinaryOp::Assignment { op: Some(arith_op) };

        let expr_parent = bin_expr.syntax().parent()?;
        assists.add(
            AssistId::quick_fix("replace-with-compound-expr"),
            Label::new("Replace with compound assignment expr".to_string()),
            diagnostic_range.range,
            |builder| {
                let make = SyntaxFactory::new();
                let mut editor = builder.make_editor(&expr_parent);

                let new_bin_expr = make.expr_bin(lhs_expr, compound_op, rhs_expr);
                editor.replace(bin_expr.syntax(), new_bin_expr.syntax().clone_for_update());

                builder.add_file_edits(file_id, editor);
            },
        );
    }

    Some(assists)
}
