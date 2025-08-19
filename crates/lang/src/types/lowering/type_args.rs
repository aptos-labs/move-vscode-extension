// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::types::inference::TypeError;
use crate::types::lowering::TyLowering;
use crate::types::substitution::Substitution;
use crate::types::ty::Ty;
use crate::types::ty::type_param::TyTypeParameter;
use base_db::SourceDatabase;
use std::collections::HashMap;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, ast};

impl TyLowering<'_> {
    pub fn type_args_substitution(
        &self,
        db: &dyn SourceDatabase,
        method_or_path: InFile<&ast::MethodOrPath>,
        generic_item: InFile<&ast::GenericElement>,
    ) -> (Substitution, Vec<TypeError>) {
        let (method_or_path_file_id, method_or_path) = method_or_path.unpack();

        let psi_subst = psi_type_args_subst(method_or_path, generic_item.value);

        let mut subst_mapping = HashMap::new();
        let mut missing_ability_errors = vec![];
        for (type_param, psi_type_arg) in psi_subst {
            let type_param = InFile::new(generic_item.file_id, type_param);
            let type_param_ty = TyTypeParameter::new(type_param.clone());
            let mut missing_abilities = vec![];
            let ty = match psi_type_arg {
                PsiTypeArg::Present(type_) => {
                    let explicit_ty = self.lower_type(type_.clone().in_file(method_or_path_file_id));

                    if let (Some(required_abilities), Some(type_arg_abilities)) =
                        (type_param_ty.abilities(db), explicit_ty.abilities(db))
                    {
                        for required_ability in required_abilities.iter() {
                            if !type_arg_abilities.contains(required_ability) {
                                missing_abilities.push(required_ability.clone());
                            }
                        }
                    }
                    if !missing_abilities.is_empty() {
                        missing_ability_errors.push(TypeError::missing_abilities(
                            type_.syntax().clone().into(),
                            explicit_ty.clone(),
                            missing_abilities,
                        ));
                    }
                    explicit_ty
                }
                PsiTypeArg::OptionalAbsent => Ty::TypeParam(type_param_ty.clone()),
                PsiTypeArg::RequiredAbsent => Ty::Unknown,
            };
            subst_mapping.insert(type_param_ty, ty);
        }

        (Substitution::new(subst_mapping), missing_ability_errors)
    }
}

fn psi_type_args_subst(
    method_or_path: &ast::MethodOrPath,
    generic_item: &ast::GenericElement,
) -> HashMap<ast::TypeParam, PsiTypeArg> {
    let is_args_optional = match method_or_path {
        ast::MethodOrPath::Path(path) => {
            let path_context = path.root_path().syntax().parent().unwrap();
            ast::Expr::can_cast(path_context.kind()) || ast::Pat::can_cast(path_context.kind())
        }
        ast::MethodOrPath::MethodCallExpr(_) => true,
    };

    // Generic arguments are optional in expression context, e.g.
    // `let a = Foo::<u8>::bar::<u16>();` can be written as `let a = Foo::bar();`
    // if it is possible to infer `u8` and `u16` during type inference

    let type_params = generic_item.type_params();
    let type_args_list = method_or_path.type_arg_list();
    if type_args_list.is_none() {
        let type_arg = if is_args_optional {
            PsiTypeArg::OptionalAbsent
        } else {
            PsiTypeArg::RequiredAbsent
        };
        return type_params.into_iter().map(|it| (it, type_arg.clone())).collect();
    }

    let mut type_args = type_args_list
        .unwrap()
        .type_arguments()
        .collect::<Vec<_>>()
        .into_iter();
    let mut subst = HashMap::new();
    for type_param in type_params {
        let type_arg = type_args.next().and_then(|it| it.type_());
        let psi_type_arg = match type_arg {
            Some(type_arg) => PsiTypeArg::Present(type_arg),
            None => PsiTypeArg::RequiredAbsent,
        };
        subst.insert(type_param, psi_type_arg);
    }
    subst
}

#[derive(Debug, Clone)]
pub enum PsiTypeArg {
    Present(ast::Type),
    RequiredAbsent,
    OptionalAbsent,
}
