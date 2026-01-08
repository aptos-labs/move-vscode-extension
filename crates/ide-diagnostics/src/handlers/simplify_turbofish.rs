use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use ide_db::Severity;
use ide_db::assist_context::LocalAssists;
use syntax::ast;
use syntax::files::{FileRange, InFile};

pub(crate) fn simplify_turbofish(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    method_call_expr: InFile<ast::MethodCallExpr>,
) -> Option<()> {
    dbg!(&method_call_expr);
    let (file_id, method) = method_call_expr.unpack_ref();
    let _ = method.type_arg_list()?;
    let coloncolon_token = method.coloncolon_token()?;
    let range = FileRange {
        file_id,
        range: coloncolon_token.text_range(),
    };
    acc.push(
        Diagnostic::new(
            DiagnosticCode::Lsp("redundant-coloncolon", Severity::Hint),
            ":: in method type arguments is deprecated",
            range,
        )
        .with_unused(true)
        .with_local_fixes(fixes(ctx, method_call_expr, range)),
    );

    Some(())
}

fn fixes(
    ctx: &DiagnosticsContext<'_>,
    method_call_expr: InFile<ast::MethodCallExpr>,
    diagnostic_range: FileRange,
) -> Option<LocalAssists> {
    let mut assists = ctx.local_assists_for_node(method_call_expr.as_ref())?;
    assists.add_fix(
        "remove-redundant-coloncolon",
        "Remove redundant ::",
        diagnostic_range.range,
        |editor| {
            if let Some(coloncolon) = method_call_expr.value.coloncolon_token() {
                editor.delete(coloncolon);
            }
        },
    );
    Some(assists)
}
