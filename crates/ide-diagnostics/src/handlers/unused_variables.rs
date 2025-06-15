use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use ide_db::{Severity, search};
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::files::InFile;
use syntax::{AstNode, ast};

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn check_unused_ident_pat<'db>(
    acc: &mut Vec<Diagnostic>,
    ctx: &'db DiagnosticsContext<'db>,
    ident_pat: InFile<ast::IdentPat>,
) -> Option<()> {
    let ident_name = ident_pat.value.name()?.as_string();
    if ident_name.starts_with("_") {
        return None;
    }

    if ident_pat.value.syntax().is_msl_context() {
        return None;
    }

    let ident_owner = ident_pat.value.ident_owner()?;
    if let ast::IdentPatOwner::Param(fun_param) = &ident_owner {
        let any_fun = fun_param.any_fun()?;
        if any_fun.is_native() || any_fun.is_uninterpreted() {
            return None;
        }
    }

    if !search::item_usages(&ctx.sema, ident_pat.clone().map_into()).at_least_one() {
        let ident_kind = ident_owner.kind();
        acc.push(Diagnostic::new(
            DiagnosticCode::Lsp("unused-variable", Severity::Warning),
            format!("Unused {ident_kind} '{ident_name}'"),
            ident_pat.file_range(),
        ));
    }

    Some(())
}
