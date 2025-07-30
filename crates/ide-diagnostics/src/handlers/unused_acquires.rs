// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use ide_db::Severity;
use ide_db::assist_context::LocalAssists;
use syntax::SyntaxKind::WHITESPACE;
use syntax::files::{FileRange, InFile, InFileExt};
use syntax::{AstNode, ast};

pub(crate) fn unused_acquires_on_inline_function(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    fun: InFile<ast::Fun>,
) -> Option<()> {
    let (file_id, fun) = fun.unpack();
    if !fun.is_inline() {
        return None;
    }
    let acquires = fun.acquires()?;
    let diag_range = FileRange {
        file_id,
        range: acquires.syntax().text_range(),
    };
    acc.push(
        Diagnostic::new(
            DiagnosticCode::Lsp("unused-acquires", Severity::WeakWarning),
            "Acquires declarations are not applicable to inline functions and should be removed",
            diag_range,
        )
        // .with_unused(true)
        .with_local_fixes(fixes(ctx, acquires.in_file(file_id), diag_range)),
    );
    Some(())
}

fn fixes(
    ctx: &DiagnosticsContext<'_>,
    acquires: InFile<ast::Acquires>,
    diagnostic_range: FileRange,
) -> Option<LocalAssists> {
    let mut assists = ctx.local_assists_for_node(acquires.as_ref())?;
    assists.add_fix(
        "remove-unused-acquires",
        "Remove acquires",
        diagnostic_range.range,
        |editor| {
            let acquires = acquires.value;
            let next_ws_sibling = acquires.syntax().next_sibling_or_token();
            editor.delete(acquires.syntax());
            // remove trailing whitespace if there is one
            if let Some(next_ws_sibling) = next_ws_sibling
                && next_ws_sibling.kind() == WHITESPACE
            {
                editor.delete(next_ws_sibling);
            }
        },
    );
    Some(assists)
}
