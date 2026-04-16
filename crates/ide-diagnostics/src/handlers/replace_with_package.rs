// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use ide_db::Severity;
use ide_db::assist_context::LocalAssists;
use syntax::ast;
use syntax::ast::AstNode;
use syntax::files::{FileRange, InFile, InFileExt};

pub(crate) fn public_package_can_be_replaced_with_package(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    fun: InFile<ast::Fun>,
) -> Option<()> {
    let (file_id, fun) = fun.unpack();
    let vis_modifier = fun.visibility_modifier()?;
    if !vis_modifier.is_public_package() {
        return None;
    }
    let range = FileRange {
        file_id,
        range: vis_modifier.syntax().text_range(),
    };
    acc.push(
        Diagnostic::new(
            DiagnosticCode::Lsp("replace-with-package", Severity::WeakWarning),
            "`public(package)` can be replaced with `package`",
            range,
        )
        .with_local_fixes(fixes(ctx, vis_modifier.in_file(file_id), range)),
    );
    Some(())
}

fn fixes(
    ctx: &DiagnosticsContext<'_>,
    vis_modifier: InFile<ast::VisibilityModifier>,
    diagnostic_range: FileRange,
) -> Option<LocalAssists> {
    let mut assists = ctx.local_assists_for_node(vis_modifier.as_ref())?;
    assists.add_fix(
        "replace-with-package",
        "Replace `public(package)` with `package`",
        diagnostic_range.range,
        |editor| {
            let vis = &vis_modifier.value;
            if let Some(public) = vis.public_token() {
                editor.delete(public);
            }
            if let Some(l_paren) = vis.l_paren_token() {
                editor.delete(l_paren);
            }
            if let Some(r_paren) = vis.r_paren_token() {
                editor.delete(r_paren);
            }
        },
    );
    Some(assists)
}
