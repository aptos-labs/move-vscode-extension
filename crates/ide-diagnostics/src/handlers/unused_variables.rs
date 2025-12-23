// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use ide_db::assist_context::LocalAssists;
use ide_db::{Severity, search};
use syntax::ast::AnyFun;
use syntax::ast::node_ext::syntax_element::SyntaxElementExt;
use syntax::ast::syntax_factory::SyntaxFactory;
use syntax::files::{FileRange, InFile};
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

    // if it's resolved to enum variant, then it can't be unused
    if ctx
        .sema
        .resolve_to_element::<ast::Variant>(ident_pat.clone())
        .is_some()
    {
        return None;
    }

    let ident_owner = ident_pat.value.ident_owner()?;
    if let ast::IdentPatOwner::Param(fun_param) = &ident_owner {
        if fun_param.is_self() {
            return None;
        }

        let any_fun = fun_param.any_fun()?;
        if any_fun.is_native() || any_fun.is_uninterpreted() {
            return None;
        }

        // skip #[test] function parameters
        if let AnyFun::Fun(fun) = any_fun
            && fun.is_test()
        {
            return None;
        }
    }

    if !search::item_usages(&ctx.sema, ident_pat.clone().map_into()).at_least_one() {
        let ident_range = ident_pat.file_range();
        let ident_kind = ident_owner.kind();
        acc.push(
            Diagnostic::new(
                DiagnosticCode::Lsp("unused-variable", Severity::Warning),
                format!("Unused {ident_kind} '{ident_name}'"),
                ident_range,
            )
            .with_local_fixes(fixes(ctx, ident_pat, ident_range)),
        );
    }

    Some(())
}

fn fixes(
    ctx: &DiagnosticsContext<'_>,
    ident_pat: InFile<ast::IdentPat>,
    diagnostic_range: FileRange,
) -> Option<LocalAssists> {
    let mut assists = ctx.local_assists_for_node(ident_pat.as_ref())?;
    let new_ident_name = format!("_{}", ident_pat.value.name()?.as_string());
    assists.add_fix(
        "rename-with-underscore-prefix",
        format!("Rename to {new_ident_name}"),
        diagnostic_range.range,
        |editor| {
            let make = SyntaxFactory::new();
            let new_ident_pat = make.ident_pat(&new_ident_name);
            editor.replace(ident_pat.value.syntax(), new_ident_pat.syntax());
            editor.add_mappings(make.finish_with_mappings());
        },
    );
    Some(assists)
}
