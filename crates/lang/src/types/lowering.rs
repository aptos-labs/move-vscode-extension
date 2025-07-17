// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

mod type_args;

use crate::loc::SyntaxLocNodeExt;
use crate::nameres;
use crate::types::has_type_params_ext::GenericItemExt;
use crate::types::substitution::ApplySubstitution;
use crate::types::ty::Ty;
use crate::types::ty::adt::TyAdt;
use crate::types::ty::integer::IntegerKind;
use crate::types::ty::range_like::TySequence;
use crate::types::ty::reference::Mutability;
use crate::types::ty::schema::TySchema;
use crate::types::ty::tuple::TyTuple;
use crate::types::ty::ty_callable::{TyCallable, TyCallableKind};
use crate::types::ty::type_param::TyTypeParameter;
use base_db::SourceDatabase;
use syntax::ast;
use syntax::ast::idents::INTEGER_IDENTS;
use syntax::files::{InFile, InFileExt};

pub struct TyLowering<'db> {
    db: &'db dyn SourceDatabase,
    msl: bool,
}

impl<'db> TyLowering<'db> {
    pub fn new(db: &'db dyn SourceDatabase, msl: bool) -> Self {
        TyLowering { db, msl }
    }

    pub fn lower_type(&self, type_: InFile<ast::Type>) -> Ty {
        self.lower_type_inner(type_).unwrap_or(Ty::Unknown)
    }

    fn lower_type_inner(&self, type_: InFile<ast::Type>) -> Option<Ty> {
        let (file_id, type_) = type_.unpack();
        match type_ {
            ast::Type::PathType(path_type) => {
                let path = path_type.path().in_file(file_id);
                let named_item = nameres::resolve_no_inf(self.db, path.clone());
                match named_item {
                    None => {
                        // can be primitive type
                        self.lower_primitive_type(path)
                    }
                    Some(named_item_entry) => named_item_entry
                        .node_loc
                        .to_ast::<ast::NamedElement>(self.db)
                        .map(|named_item| self.lower_path(path.map_into(), named_item)),
                }
            }
            ast::Type::RefType(ref_type) => {
                let is_mut = ref_type.is_mut();
                let inner_ty = ref_type
                    .type_()
                    .map(|inner_type| self.lower_type(inner_type.in_file(file_id)))
                    .unwrap_or(Ty::Unknown);
                Some(Ty::new_reference(inner_ty, Mutability::new(is_mut)))
            }
            ast::Type::TupleType(tuple_type) => {
                let inner_tys = tuple_type
                    .types()
                    .map(|it| self.lower_type(it.in_file(file_id)))
                    .collect::<Vec<_>>();
                Some(Ty::Tuple(TyTuple::new(inner_tys)))
            }
            ast::Type::UnitType(_) => Some(Ty::Unit),
            ast::Type::ParenType(paren_type) => {
                self.lower_type_inner(paren_type.type_()?.in_file(file_id))
            }
            ast::Type::LambdaType(lambda_type) => {
                let param_tys = lambda_type
                    .param_types()
                    .into_iter()
                    .map(|it| self.lower_type(it.in_file(file_id)))
                    .collect();
                let ret_ty = lambda_type
                    .return_type()
                    .map(|it| self.lower_type(it.in_file(file_id)))
                    .unwrap_or(Ty::Unit);
                Some(Ty::Callable(TyCallable::new(
                    param_tys,
                    ret_ty,
                    TyCallableKind::Lambda(Some(lambda_type.loc(file_id))),
                )))
            }
        }
    }

    pub fn lower_path(
        &self,
        method_or_path: InFile<ast::MethodOrPath>,
        named_item: InFile<impl Into<ast::NamedElement>>,
    ) -> Ty {
        use syntax::SyntaxKind::*;

        let named_item = named_item.map(|it| it.into());
        let path_ty = match named_item.kind() {
            TYPE_PARAM => {
                let type_param = named_item.clone().cast_into::<ast::TypeParam>().unwrap();
                Ty::TypeParam(TyTypeParameter::new(type_param))
            }
            STRUCT | ENUM => {
                let item = named_item.clone().cast_into::<ast::StructOrEnum>().unwrap();
                Ty::Adt(TyAdt::new(item))
            }
            SCHEMA => {
                let item = named_item.clone().cast_into::<ast::Schema>().unwrap();
                Ty::Schema(TySchema::new(item))
            }
            FUN | SPEC_FUN | SPEC_INLINE_FUN => {
                let fun = named_item.clone().cast_into::<ast::AnyFun>().unwrap();
                let ty_callable = self.lower_any_function(fun);
                Ty::Callable(ty_callable)
            }
            VARIANT => {
                let variant = named_item.clone().cast_into::<ast::Variant>().unwrap();
                let enum_ = variant.map(|it| it.enum_());
                let (file_id, method_or_path) = method_or_path.clone().unpack();
                let Some(enum_path) = method_or_path
                    .path()
                    .expect("MethodCallExpr cannot be resolved to Variant")
                    .qualifier()
                else {
                    return Ty::Unknown;
                };
                self.lower_path(enum_path.in_file(file_id).map_into(), enum_)
            }
            _ => Ty::Unknown,
        };

        // adds associations of ?Element -> (type of ?Element from explicitly set types)
        // Option<u8>: ?Element -> u8
        // Option: ?Element -> ?Element
        if let Some(generic_item) = named_item.cast_into::<ast::GenericElement>() {
            let type_args_subst = self.type_args_substitution(method_or_path, generic_item);
            return path_ty.substitute(&type_args_subst);
        }

        path_ty
    }

    pub fn lower_type_owner(&self, type_owner: InFile<ast::TypeOwner>) -> Option<Ty> {
        type_owner
            .and_then(|it| it.type_())
            .map(|type_| self.lower_type(type_))
    }

    pub fn lower_any_function(&self, any_fun: InFile<ast::AnyFun>) -> TyCallable {
        let item_subst = any_fun.ty_type_params_subst();
        let (file_id, any_fun) = any_fun.unpack();
        let param_types = any_fun
            .params()
            .into_iter()
            .map(|it| {
                it.type_()
                    .map(|t| self.lower_type(t.in_file(file_id)))
                    .unwrap_or(Ty::Unknown)
            })
            .collect();
        let ret_type = self.lower_ret_type(any_fun.ret_type().map(|t| t.in_file(file_id)));
        TyCallable::new(
            param_types,
            ret_type,
            TyCallableKind::named(item_subst, Some(any_fun.loc(file_id))),
        )
    }

    fn lower_ret_type(&self, ret_type: Option<InFile<ast::RetType>>) -> Ty {
        let Some(ret_type) = ret_type else {
            return Ty::Unit;
        };
        ret_type
            .and_then(|it| it.type_())
            .map(|t| self.lower_type(t))
            .unwrap_or(Ty::Unknown)
    }

    fn lower_primitive_type(&self, path: InFile<ast::Path>) -> Option<Ty> {
        let (file_id, path) = path.unpack();
        let path_name = path.reference_name()?;
        if self.msl && INTEGER_IDENTS.contains(&path_name.as_str()) {
            return Some(Ty::Num);
        }
        let ty = match path_name.as_str() {
            "u8" => Ty::Integer(IntegerKind::U8),
            "u16" => Ty::Integer(IntegerKind::U16),
            "u32" => Ty::Integer(IntegerKind::U32),
            "u64" => Ty::Integer(IntegerKind::U64),
            "u128" => Ty::Integer(IntegerKind::U128),
            "u256" => Ty::Integer(IntegerKind::U256),
            "num" => Ty::Num,
            "bv" => Ty::Bv,
            "bool" => Ty::Bool,
            "signer" => Ty::Signer,
            "address" => Ty::Address,
            "vector" => {
                let first_arg_type = path.type_args().first().and_then(|it| it.type_());
                let first_arg_ty = first_arg_type
                    .map(|it| self.lower_type(it.in_file(file_id)))
                    .unwrap_or(Ty::Unknown);
                Ty::new_vector(first_arg_ty)
            }
            "range" => {
                let first_arg_type = path.type_args().first().and_then(|it| it.type_());
                let first_arg_ty = first_arg_type
                    .map(|it| self.lower_type(it.in_file(file_id)))
                    .unwrap_or(Ty::Unknown);
                Ty::Seq(TySequence::Range(Box::new(first_arg_ty)))
            }
            _ => {
                return None;
            }
        };
        Some(ty)
    }
}
