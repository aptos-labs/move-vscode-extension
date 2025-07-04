// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use ide_db::RootDatabase;
use lang::Semantics;
use lang::node_ext::call_ext;
use lang::types::lowering::TyLowering;
use lang::types::ty::Ty;
use lang::types::ty::integer::IntegerKind;
use lang::types::ty::ty_callable::TyCallable;
use stdx::format_to;
use syntax::files::{FilePosition, InFile, InFileExt};
use syntax::{
    AstNode, Direction, NodeOrToken, SyntaxToken, T, TextRange, TextSize, algo, ast, match_ast,
};

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
                _ => (),
            }
        }
    }

    None
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
    let (any_call_expr, active_parameter) = callable_for_arg_list(arg_list, token.text_range().start())?;

    let mut res = SignatureHelp {
        signature: String::new(),
        parameters: vec![],
        active_parameter,
    };

    let db = sema.db;
    let ty_lowering = TyLowering::new(db, any_call_expr.is_msl());
    let call_ty = sema.get_call_expr_type(&any_call_expr);

    let mut fn_params = vec![];
    let (callee_file_id, callee_kind) = call_ext::callee_kind(sema, &any_call_expr)?.unpack();
    match callee_kind {
        call_ext::CalleeKind::Function(fun) => {
            for (i, param) in fun.params().into_iter().enumerate() {
                if i == 0 && matches!(any_call_expr.value, ast::AnyCallExpr::MethodCallExpr(_)) {
                    continue;
                }
                let param_name = Some(param.ident_name());
                let type_ = param.type_().map(|it| it.in_file(callee_file_id));
                fn_params.push(FnParam {
                    name: param_name,
                    ty: get_call_param_ty(type_, i, &ty_lowering, call_ty.as_ref()),
                });
            }
        }
        call_ext::CalleeKind::AssertMacro => {
            fn_params.push(FnParam {
                name: Some("_".to_string()),
                ty: Some(Ty::Bool),
            });
            fn_params.push(FnParam {
                name: Some("err".to_string()),
                ty: Some(Ty::Integer(IntegerKind::U64)),
            });
        }
        call_ext::CalleeKind::TupleStruct(s) => {
            for (i, tuple_field) in s.tuple_fields().iter().enumerate() {
                let type_ = tuple_field.type_().map(|it| it.in_file(callee_file_id));
                fn_params.push(FnParam {
                    name: None,
                    ty: get_call_param_ty(type_, i, &ty_lowering, call_ty.as_ref()),
                });
            }
        }
        call_ext::CalleeKind::TupleEnumVariant(s) => {
            for (i, tuple_field) in s.tuple_fields().iter().enumerate() {
                let type_ = tuple_field.type_().map(|it| it.in_file(callee_file_id));
                fn_params.push(FnParam {
                    name: None,
                    ty: get_call_param_ty(type_, i, &ty_lowering, call_ty.as_ref()),
                });
            }
        }
    }

    if fn_params.is_empty() {
        res.signature = "<no arguments>".to_string();
        return Some(res);
    }

    for fn_param in fn_params {
        let mut p = String::new();
        if let Some(name) = fn_param.name {
            format_to!(p, "{}: ", name);
        }
        format_to!(
            p,
            "{}",
            fn_param
                .ty
                .map(|it| sema.render_ty_truncated(&it, callee_file_id))
                .unwrap_or_default()
        );
        res.push_param(&p);
    }

    Some(res)
}

fn get_call_param_ty(
    type_: Option<InFile<ast::Type>>,
    i: usize,
    ty_lowering: &TyLowering,
    call_ty: Option<&TyCallable>,
) -> Option<Ty> {
    let mut param_ty = None;
    if let Some(param_type) = type_ {
        let pty = call_ty.and_then(|it| it.param_types.get(i)).cloned();
        param_ty = pty.or_else(|| Some(ty_lowering.lower_type(param_type)))
    }
    param_ty
}

pub fn callable_for_arg_list(
    arg_list: InFile<ast::ValueArgList>,
    at_offset: TextSize,
) -> Option<(InFile<ast::AnyCallExpr>, Option<usize>)> {
    let (file_id, arg_list) = arg_list.unpack();

    debug_assert!(arg_list.syntax().text_range().contains(at_offset));
    let callable = arg_list.syntax().parent().and_then(ast::AnyCallExpr::cast)?;
    let active_param = callable.value_arg_list().map(|arg_list| {
        arg_list
            .syntax()
            .children_with_tokens()
            .filter_map(NodeOrToken::into_token)
            .filter(|t| t.kind() == T![,])
            .take_while(|t| t.text_range().start() <= at_offset)
            .count()
    });
    Some((callable.in_file(file_id), active_param))
}
