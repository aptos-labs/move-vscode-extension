use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use ide_db::Severity;
use ide_db::assist_context::Assists;
use ide_db::assists::AssistId;
use ide_db::label::Label;
use syntax::SyntaxKind::WHITESPACE;
use syntax::files::{FileRange, InFile};
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
        .with_fixes(fixes(ctx, acquires, diag_range)),
    );
    Some(())
}

fn fixes(
    ctx: &DiagnosticsContext<'_>,
    acquires: ast::Acquires,
    diagnostic_range: FileRange,
) -> Option<Assists> {
    let fun = acquires.fun();
    let mut assists = Assists::new(diagnostic_range.file_id, ctx.resolve.clone());
    assists.add(
        AssistId::quick_fix("remove-unused-acquires"),
        Label::new("Remove acquires".to_string()),
        diagnostic_range.range,
        |builder| {
            let next_ws_sibling = acquires.syntax().next_sibling_or_token();
            let mut edits = builder.make_editor(fun.syntax());
            edits.delete(acquires.syntax());
            // remove trailing whitespace if there is one
            if let Some(next_ws_sibling) = next_ws_sibling
                && next_ws_sibling.kind() == WHITESPACE
            {
                edits.delete(next_ws_sibling);
            }

            builder.add_file_edits(diagnostic_range.file_id, edits);
        },
    );
    Some(assists)
}
