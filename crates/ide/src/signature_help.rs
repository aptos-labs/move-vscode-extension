// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use ide_db::active_parameter::{call_expr_for_arg_list, generic_item_for_type_arg_list};
use ide_db::{RootDatabase, active_parameter};
use itertools::Itertools;
use lang::Semantics;
use lang::types::ty::Ty;
use stdx::format_to;
use syntax::files::{FilePosition, InFile, InFileExt};
use syntax::{AstNode, Direction, SyntaxToken, TextRange, TextSize, algo, ast, match_ast};

/// Contains information about an item signature as seen from a use site.
///
/// This includes the "active parameter", which is the parameter whose value is currently being
/// edited.
#[derive(Debug)]
pub struct SignatureHelp {
    pub signature: String,
    pub active_parameter: Option<usize>,
    parameters: Vec<TextRange>,
}

impl SignatureHelp {
    pub fn parameter_labels(&self) -> impl Iterator<Item = &str> + '_ {
        self.parameters.iter().map(move |&it| &self.signature[it])
    }

    pub fn parameter_ranges(&self) -> &[TextRange] {
        &self.parameters
    }

    pub fn parameter_range(&self, n: usize) -> Option<TextRange> {
        self.parameters.get(n).cloned()
    }

    fn push_param(&mut self, param: &str) {
        if !self.parameters.is_empty() {
            self.signature.push_str(", ");
        }
        let start = TextSize::of(&self.signature);
        self.signature.push_str(param);
        let end = TextSize::of(&self.signature);
        self.parameters.push(TextRange::new(start, end))
    }
}

/// Computes parameter information for the given position.
pub(crate) fn signature_help(
    db: &RootDatabase,
    FilePosition { file_id, offset }: FilePosition,
) -> Option<SignatureHelp> {
    let sema = Semantics::new(db, file_id);
    let file = sema.parse(file_id);
    let file = file.syntax();
    let token = file
        .token_at_offset(offset)
        .left_biased()
        // if the cursor is sandwiched between two space tokens and the call is unclosed
        // this prevents us from leaving the CallExpr
        .and_then(|tok| algo::skip_trivia_token(tok, Direction::Prev))?;

    for node in token.parent_ancestors() {
        match_ast! {
            match node {
                ast::ValueArgList(arg_list) => {
                    let cursor_outside = arg_list.r_paren_token().as_ref() == Some(&token);
                    if cursor_outside {
                        continue;
                    }
                    return signature_help_for_call(&sema, arg_list.in_file(file_id), token);
                },
                ast::TypeArgList(arg_list) => {
                    let cursor_outside = arg_list.r_angle_token().as_ref() == Some(&token);
                    if cursor_outside {
                        continue;
                    }
                    return signature_help_for_type_args(&sema, arg_list.in_file(file_id), token);
                },
                ast::StructLit(struct_lit) => {
                    let cursor_outside = struct_lit.struct_lit_field_list().and_then(|list| list.r_curly_token()).as_ref() == Some(&token);
                    if cursor_outside {
                        continue;
                    }
                    return signature_help_for_struct_lit(&sema, struct_lit.in_file(file_id), token);
                },
                _ => (),
            }
        }
    }

    None
}

fn signature_help_for_type_args(
    sema: &Semantics<'_, RootDatabase>,
    type_arg_list: InFile<ast::TypeArgList>,
    token: SyntaxToken,
) -> Option<SignatureHelp> {
    let (generic_item, active_param) = generic_item_for_type_arg_list(sema, type_arg_list, &token)?;
    let mut res = SignatureHelp {
        signature: String::new(),
        parameters: vec![],
        active_parameter: Some(active_param),
    };

    let type_params = generic_item.value.type_params();
    if type_params.is_empty() {
        res.signature = "<no arguments>".to_string();
        return Some(res);
    }

    let mut buf = String::new();
    for type_param in type_params {
        buf.clear();
        let param_name = type_param.name().map(|it| it.as_string()).unwrap_or_default();
        format_to!(buf, "{}", param_name);
        let ability_bounds = type_param.ability_bounds();
        if !ability_bounds.is_empty() {
            let bounds = ability_bounds.iter().map(|it| it.to_string()).join(" + ");
            format_to!(buf, ": {}", bounds);
        }
        res.push_param(&buf);
    }

    Some(res)
}

fn signature_help_for_struct_lit(
    sema: &Semantics<'_, RootDatabase>,
    struct_lit: InFile<ast::StructLit>,
    token: SyntaxToken,
) -> Option<SignatureHelp> {
    let (fields_owner, active_field_name) = active_parameter::fields_owner_for_struct_lit(
        sema,
        struct_lit.clone(),
        token.text_range().start(),
    )?;
    let (file_id, fields_owner) = fields_owner.unpack();
    let named_fields = fields_owner.named_fields();

    let mut res = SignatureHelp {
        signature: String::new(),
        parameters: vec![],
        active_parameter: Some(named_fields.len()),
    };
    if named_fields.is_empty() {
        res.signature = "<no fields>".to_string();
        return Some(res);
    }

    let msl = struct_lit.is_msl();
    for (i, named_field) in named_fields.iter().enumerate() {
        let field_name = named_field.field_name().as_string();
        if active_field_name.as_ref().is_some_and(|it| it == &field_name) {
            res.active_parameter = Some(i);
        }
        let mut field_text = String::new();
        format_to!(field_text, "{}", field_name);
        if let Some(field_type) = named_field.type_().map(|it| it.in_file(file_id)) {
            format_to!(
                field_text,
                ": {}",
                sema.render_ty_truncated(&sema.lower_type(field_type, msl), file_id)
            );
        }
        res.push_param(&field_text);
    }

    Some(res)
}

#[derive(Debug)]
struct FnParam {
    name: Option<String>,
    ty: Option<Ty>,
}

fn signature_help_for_call(
    sema: &Semantics<'_, RootDatabase>,
    arg_list: InFile<ast::ValueArgList>,
    token: SyntaxToken,
) -> Option<SignatureHelp> {
    let (any_call_expr, active_parameter) =
        call_expr_for_arg_list(arg_list, token.text_range().start())?;

    let callable = sema.callable(&any_call_expr)?;

    let mut res = SignatureHelp {
        signature: String::new(),
        parameters: vec![],
        active_parameter,
    };

    let callable_params = callable.params()?;
    if callable_params.is_empty() {
        res.signature = "<no arguments>".to_string();
        return Some(res);
    }

    for callable_param in callable_params {
        let mut p = String::new();
        if let Some(name) = callable_param.name {
            format_to!(p, "{}: ", name);
        }
        format_to!(
            p,
            "{}",
            callable_param
                .ty
                .map(|it| sema.render_ty_truncated(&it, callable.file_id()))
                .unwrap_or_default()
        );
        res.push_param(&p);
    }

    Some(res)
}
