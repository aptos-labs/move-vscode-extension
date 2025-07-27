// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use ide_db::Severity;
use std::collections::HashSet;
use syntax::ast;
use syntax::files::InFile;

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn missing_fields_in_struct_lit(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    struct_lit: InFile<ast::StructLit>,
) -> Option<()> {
    let lit_path = struct_lit.as_ref().map(|it| it.path());
    let fields_owner = ctx
        .sema
        .resolve_to_element::<ast::FieldsOwner>(lit_path.clone())?;
    let declared_field_names = fields_owner.value.named_field_names();
    let provided_field_names = struct_lit
        .value
        .fields()
        .iter()
        .filter_map(|it| it.field_name())
        .collect::<HashSet<_>>();

    let mut missing_fields = vec![];
    for declared_field_name in declared_field_names {
        if !provided_field_names.contains(&declared_field_name) {
            missing_fields.push(declared_field_name);
        }
    }

    let lit_ident_token = lit_path.syntax_text();
    let error_message = match missing_fields.len() {
        0 => {
            return None;
        }
        1 => {
            let missing_field_name = missing_fields.pop().unwrap();
            format!(
                "Missing field for `{}` initializer: `{}`",
                lit_ident_token, missing_field_name
            )
        }
        _ => {
            let missing_field_names = missing_fields
                .iter()
                .map(|it| format!("`{it}`"))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "Missing fields for `{}` initializer: {}",
                lit_ident_token, missing_field_names
            )
        }
    };

    acc.push(Diagnostic::new(
        DiagnosticCode::Lsp("missing-lit-fields", Severity::Error),
        error_message,
        lit_path.file_range(),
    ));
    Some(())
}

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn missing_fields_in_struct_pat(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    struct_pat: InFile<ast::StructPat>,
) -> Option<()> {
    if struct_pat.value.has_rest_pat() {
        return None;
    }
    let pat_path = struct_pat.as_ref().map(|it| it.path());
    let fields_owner = ctx
        .sema
        .resolve_to_element::<ast::FieldsOwner>(pat_path.clone())?;
    let declared_field_names = fields_owner.value.named_field_names();
    let provided_field_names = struct_pat
        .value
        .fields()
        .iter()
        .filter_map(|it| it.field_name())
        .collect::<HashSet<_>>();

    let mut missing_fields = vec![];
    for declared_field_name in declared_field_names {
        if !provided_field_names.contains(&declared_field_name) {
            missing_fields.push(declared_field_name);
        }
    }

    let field_owner_type = match fields_owner.value {
        ast::FieldsOwner::Struct(_) => "Struct",
        ast::FieldsOwner::Variant(_) => "Enum variant",
    };

    let error_message = match missing_fields.len() {
        0 => {
            return None;
        }
        1 => {
            let missing_field_name = missing_fields.pop().unwrap();
            format!("{field_owner_type} pattern does not mention field `{missing_field_name}`")
        }
        _ => {
            let missing_field_names = missing_fields
                .iter()
                .map(|it| format!("`{it}`"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{field_owner_type} pattern does not mention fields {missing_field_names}")
        }
    };

    acc.push(Diagnostic::new(
        DiagnosticCode::Lsp("missing-pat-fields", Severity::Error),
        error_message,
        struct_pat.file_range(),
    ));

    Some(())
}

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn missing_fields_in_tuple_struct_pat(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    tuple_struct_pat: InFile<ast::TupleStructPat>,
) -> Option<()> {
    if tuple_struct_pat.value.has_rest_pat() {
        return None;
    }
    let pat_path = tuple_struct_pat.as_ref().map(|it| it.path());
    let fields_owner = ctx
        .sema
        .resolve_to_element::<ast::FieldsOwner>(pat_path.clone())?;

    let n_declared = fields_owner.value.tuple_fields().len();
    let n_provided = tuple_struct_pat.value.fields().collect::<Vec<_>>().len();

    if n_provided < n_declared {
        let owner_type = match fields_owner.value {
            ast::FieldsOwner::Struct(_) => "Struct",
            ast::FieldsOwner::Variant(_) => "Enum variant",
        };
        let error_message = format!(
            "{owner_type} pattern does not match its declaration: expected {n_declared} fields, found {n_provided}"
        );
        acc.push(Diagnostic::new(
            DiagnosticCode::Lsp("missing-tuple-pat-fields", Severity::Error),
            error_message,
            tuple_struct_pat.file_range(),
        ));
    }

    Some(())
}
