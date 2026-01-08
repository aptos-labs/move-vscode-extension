use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use ide_db::Severity;
use ide_db::assist_context::LocalAssists;
use syntax::ast;
use syntax::files::{FileRange, InFile, InFileExt};

pub(crate) fn simplify_turbofish(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    method_call_expr: InFile<ast::MethodCallExpr>,
) -> Option<()> {
    let (file_id, method_call_expr) = method_call_expr.unpack_ref();
    let type_arg_list = method_call_expr.type_arg_list()?;
    let coloncolon_token = type_arg_list.coloncolon_token()?;
    let range = FileRange {
        file_id,
        range: coloncolon_token.text_range(),
    };
    acc.push(
        Diagnostic::new(
            DiagnosticCode::Lsp("redundant-coloncolon", Severity::Hint),
            "`::` in method type arguments is deprecated",
            range,
        )
        .with_unused(true)
        .with_local_fixes(fixes(ctx, type_arg_list.in_file(file_id), range)),
    );

    Some(())
}

fn fixes(
    ctx: &DiagnosticsContext<'_>,
    type_arg_list: InFile<ast::TypeArgList>,
    diagnostic_range: FileRange,
) -> Option<LocalAssists> {
    let mut assists = ctx.local_assists_for_node(type_arg_list.as_ref())?;
    assists.add_fix(
        "remove-redundant-coloncolon",
        "Remove redundant ::",
        diagnostic_range.range,
        |editor| {
            if let Some(coloncolon) = type_arg_list.value.coloncolon_token() {
                editor.delete(coloncolon);
            }
        },
    );
    Some(assists)
}
