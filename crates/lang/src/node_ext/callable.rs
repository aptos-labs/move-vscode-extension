// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::types::ty::Ty;
use crate::types::ty::integer::IntegerKind;
use crate::types::ty::ty_callable::{TyCallable, TyCallableKind};
use base_db::SourceDatabase;
use syntax::ast;
use syntax::files::{InFile, InFileExt};
use vfs::FileId;

#[derive(Debug)]
pub struct Callable {
    file_id: FileId,
    call_expr: InFile<ast::AnyCallExpr>,
    ty: Option<TyCallable>,
    pub callable_item: CallableItem,
}

impl Callable {
    pub fn new(
        db: &dyn SourceDatabase,
        any_call_expr: InFile<ast::AnyCallExpr>,
        callable_ty: Option<TyCallable>,
    ) -> Option<Self> {
        if matches!(any_call_expr.value, ast::AnyCallExpr::AssertMacroExpr(_)) {
            return Some(Callable {
                file_id: db.builtins_file_id()?.data(db),
                call_expr: any_call_expr,
                ty: None,
                callable_item: CallableItem::AssertMacro,
            });
        }

        let callable_ty = callable_ty?;
        let (file_id, callable_item) = match &callable_ty.kind {
            TyCallableKind::Named(Some(callable_loc)) => {
                let (file_id, named_element) = callable_loc.to_ast::<ast::NamedElement>(db)?.unpack();
                match named_element {
                    ast::NamedElement::Fun(fun) => Some(CallableItem::Function(fun.into())),
                    ast::NamedElement::SpecFun(fun) => Some(CallableItem::Function(fun.into())),
                    ast::NamedElement::SpecInlineFun(fun) => Some(CallableItem::Function(fun.into())),
                    ast::NamedElement::Struct(s) => Some(CallableItem::TupleStruct(s)),
                    ast::NamedElement::Variant(v) => Some(CallableItem::TupleEnumVariant(v)),
                    _ => None,
                }
                .map(|it| it.in_file(file_id))
            }
            TyCallableKind::Lambda(Some(lambda_loc)) => {
                if let Some(lambda_expr) = lambda_loc.to_ast::<ast::LambdaExpr>(db) {
                    Some(lambda_expr.map(|it| CallableItem::LambdaExpr(it)))
                } else if let Some(lambda_type) = lambda_loc.to_ast::<ast::LambdaType>(db) {
                    Some(lambda_type.map(|it| CallableItem::LambdaType(it)))
                } else {
                    None
                }
            }
            _ => None,
        }?
        .unpack();

        Some(Callable {
            file_id,
            call_expr: any_call_expr,
            ty: Some(callable_ty),
            callable_item,
        })
    }

    pub fn file_id(&self) -> FileId {
        self.file_id
    }

    pub fn call_file_id(&self) -> FileId {
        self.call_expr.file_id
    }

    pub fn params(&self) -> Option<Vec<CallableParam>> {
        if matches!(self.callable_item, CallableItem::AssertMacro) {
            return Some(vec![
                CallableParam {
                    name: Some("_".to_string()),
                    ty: Some(Ty::Bool),
                    kind: CallableParamKind::AssertParam,
                },
                CallableParam {
                    name: Some("err".to_string()),
                    ty: Some(Ty::Integer(IntegerKind::U64)),
                    kind: CallableParamKind::AssertParam,
                },
            ]);
        }
        let callable_ty = self.ty.as_ref()?;
        let mut params = vec![];
        match &self.callable_item {
            CallableItem::Function(fun) => {
                for (i, param) in fun.params().into_iter().enumerate() {
                    if i == 0 && matches!(self.call_expr.value, ast::AnyCallExpr::MethodCallExpr(_)) {
                        continue;
                    }
                    let param_name = Some(param.ident_name());
                    params.push(CallableParam {
                        name: param_name,
                        ty: callable_ty.param_types.get(i).cloned(),
                        kind: CallableParamKind::FunParam(param),
                    });
                }
            }
            CallableItem::TupleStruct(s) => {
                for (i, tuple_field) in s.tuple_fields().into_iter().enumerate() {
                    params.push(CallableParam {
                        name: None,
                        ty: callable_ty.param_types.get(i).cloned(),
                        kind: CallableParamKind::TupleField(tuple_field),
                    });
                }
            }
            CallableItem::TupleEnumVariant(s) => {
                for (i, tuple_field) in s.tuple_fields().into_iter().enumerate() {
                    params.push(CallableParam {
                        name: None,
                        ty: callable_ty.param_types.get(i).cloned(),
                        kind: CallableParamKind::TupleField(tuple_field),
                    });
                }
            }
            CallableItem::LambdaType(lambda_type) => {
                for (i, lambda_expr_param) in lambda_type.lambda_type_params().into_iter().enumerate() {
                    params.push(CallableParam {
                        name: None,
                        ty: callable_ty.param_types.get(i).cloned(),
                        kind: CallableParamKind::LambdaTypeParam(lambda_expr_param),
                    });
                }
            }
            CallableItem::LambdaExpr(lambda_expr) => {
                for (i, lambda_expr_param) in lambda_expr.params().into_iter().enumerate() {
                    params.push(CallableParam {
                        name: lambda_expr_param.name_as_string(),
                        ty: callable_ty.param_types.get(i).cloned(),
                        kind: CallableParamKind::LambdaParam(lambda_expr_param),
                    });
                }
            }
            CallableItem::AssertMacro => unreachable!(),
        }
        Some(params)
    }
}

#[derive(Debug)]
pub enum CallableItem {
    Function(ast::AnyFun),
    TupleStruct(ast::Struct),
    TupleEnumVariant(ast::Variant),
    AssertMacro,
    LambdaExpr(ast::LambdaExpr),
    LambdaType(ast::LambdaType),
}

#[derive(Debug)]
pub struct CallableParam {
    pub name: Option<String>,
    pub ty: Option<Ty>,
    pub kind: CallableParamKind,
}

#[derive(Debug)]
pub enum CallableParamKind {
    FunParam(ast::Param),
    LambdaParam(ast::LambdaParam),
    LambdaTypeParam(ast::LambdaTypeParam),
    TupleField(ast::TupleField),
    AssertParam,
}

impl CallableParamKind {
    pub fn into_fun_param(self) -> Option<ast::Param> {
        match self {
            CallableParamKind::FunParam(param) => Some(param),
            _ => None,
        }
    }
}
