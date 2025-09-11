// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

mod type_args;

use crate::loc::SyntaxLocNodeExt;
use crate::nameres;
use crate::types::has_type_params_ext::GenericItemExt;
use crate::types::inference::TypeError;
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
use crate::types::ty_db;
use base_db::SourceDatabase;
use syntax::ast;
use syntax::files::{InFile, InFileExt};

pub struct TyLowering<'db> {
    db: &'db dyn SourceDatabase,
    msl: bool,
}

impl<'db> TyLowering<'db> {
    pub fn new(db: &'db dyn SourceDatabase, msl: bool) -> Self {
        TyLowering { db, msl }
    }

    pub fn lower_type_inner(&self, type_: InFile<ast::Type>) -> Option<Ty> {
        let (file_id, type_) = type_.unpack();
        match type_ {
            ast::Type::PathType(path_type) => {
                let path = path_type.path().in_file(file_id);
                let named_item = nameres::resolve_no_inf(self.db, path.clone());
                match named_item {
                    None => {
                        // can still be primitive type
                        ty_db::lower_primitive_type(self.db, path, self.msl)
                    }
                    Some(named_item_entry) => {
                        let named_element =
                            named_item_entry.node_loc.to_ast::<ast::NamedElement>(self.db)?;
                        let (path_type_ty, _) = self.lower_path(path.map_into(), named_element);
                        // todo: ability checks in types
                        Some(path_type_ty)
                    }
                }
            }
            ast::Type::RefType(ref_type) => {
                let is_mut = ref_type.is_mut();
                let inner_ty = ref_type
                    .type_()
                    .map(|inner_type| ty_db::lower_type(self.db, inner_type.in_file(file_id), self.msl))
                    .unwrap_or(Ty::Unknown);
                Some(Ty::new_reference(inner_ty, Mutability::new(is_mut)))
            }
            ast::Type::TupleType(tuple_type) => {
                let inner_tys = tuple_type
                    .types()
                    .map(|inner_type| ty_db::lower_type(self.db, inner_type.in_file(file_id), self.msl))
                    .collect::<Vec<_>>();
                Some(Ty::Tuple(TyTuple::new(inner_tys)))
            }
            ast::Type::UnitType(_) => Some(Ty::Unit),
            ast::Type::ParenType(paren_type) => {
                let paren_ty = paren_type.type_()?.in_file(file_id);
                ty_db::try_lower_type(self.db, paren_ty, self.msl)
            }
            ast::Type::LambdaType(lambda_type) => {
                let param_tys = lambda_type
                    .param_types()
                    .into_iter()
                    .map(|it| ty_db::lower_type(self.db, it.in_file(file_id), self.msl))
                    .collect();
                let ret_ty = lambda_type
                    .return_type()
                    .map(|it| ty_db::lower_type(self.db, it.in_file(file_id), self.msl))
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
    ) -> (Ty, Vec<TypeError>) {
        let _p = tracing::debug_span!("lower_path").entered();

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
                let ty_callable = ty_db::lower_function(self.db, fun, self.msl);
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
                    return (Ty::Unknown, vec![]);
                };
                let (variant_path_ty, _) = self.lower_path(enum_path.in_file(file_id).map_into(), enum_);
                // todo: ability checks for enum variants
                variant_path_ty
            }
            _ => Ty::Unknown,
        };

        // adds associations of ?Element -> (type of ?Element from explicitly set types)
        // Option<u8>: ?Element -> u8
        // Option: ?Element -> ?Element
        if let Some(generic_item) = named_item.cast_into::<ast::GenericElement>() {
            let (type_args_subst, type_errors) = type_args::type_args_substitution(
                self.db,
                self.msl,
                method_or_path.as_ref(),
                generic_item.as_ref(),
            );
            return (path_ty.substitute(&type_args_subst), type_errors);
        }

        (path_ty, vec![])
    }
}
