use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use ide_db::Severity;
use std::ops::Add;
use syntax::ast::HasAttrs;
use syntax::files::{FileRange, InFile};
use syntax::{AstNode, TextRange, TextSize, ast};

pub(crate) fn spec_fun_requires_return_type(
    acc: &mut Vec<Diagnostic>,
    _ctx: &DiagnosticsContext<'_>,
    spec_fun: InFile<ast::SpecFun>,
) -> Option<()> {
    let (file_id, spec_fun) = spec_fun.unpack();
    if spec_fun.ret_type().is_some() {
        return None;
    }
    let start = spec_fun
        .param_list()
        .map(|it| it.syntax().text_range().end())
        .or(spec_fun
            .spec_block()
            .map(|block| block.syntax().text_range().start()))
        .unwrap_or(spec_fun.syntax().text_range().start());
    let end = spec_fun
        .spec_block()
        .map(|it| it.syntax().text_range().start().add(TextSize::new(1)))
        .unwrap_or(spec_fun.syntax().text_range().end());
    acc.push(Diagnostic::new(
        DiagnosticCode::Lsp("spec-fun-required-return-type", Severity::Error),
        "Spec function requires return type",
        FileRange {
            file_id,
            range: TextRange::new(start, end),
        },
    ));
    Some(())
}

pub(crate) fn entry_fun_cannot_have_return_type(
    acc: &mut Vec<Diagnostic>,
    _ctx: &DiagnosticsContext<'_>,
    fun: InFile<ast::Fun>,
) -> Option<()> {
    let (file_id, fun) = fun.unpack();
    if !fun.is_entry() {
        return None;
    }
    if fun.is_test() || fun.is_test_only() {
        return None;
    }
    match fun.ret_type() {
        None => {
            return None;
        }
        Some(return_type) => acc.push(Diagnostic::new(
            DiagnosticCode::Lsp("entry-fun-cannot-return-values", Severity::Error),
            "Entry functions cannot return values",
            FileRange {
                file_id,
                range: return_type.syntax().text_range(),
            },
        )),
    }
    Some(())
}
