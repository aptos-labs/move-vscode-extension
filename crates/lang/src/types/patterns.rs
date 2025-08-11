// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::nameres::scope::{ScopeEntry, ScopeEntryExt, VecExt};
use crate::types::inference::ast_walker::TypeAstWalker;
use crate::types::inference::{TyVarIndex, TypeError};
use crate::types::patterns::BindingMode::{BindByReference, BindByValue};
use crate::types::substitution::{ApplySubstitution, empty_substitution};
use crate::types::ty::Ty;
use crate::types::ty::reference::Mutability;
use crate::types::ty::tuple::TyTuple;
use std::{cmp, iter};
use syntax::SyntaxKind;
use syntax::ast::StructOrEnum;
use syntax::ast::node_ext::struct_pat_field::PatFieldKind;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, ast};

impl TypeAstWalker<'_, '_> {
    pub fn collect_pat_bindings(&mut self, pat: ast::Pat, ty: Ty, def_bm: BindingMode) -> Option<()> {
        match pat.clone() {
            ast::Pat::PathPat(path_pat) => {
                let named_item = self.ctx.resolve_path_cached(path_pat.path(), None);
                let named_item_kind = named_item.map(|it| it.kind());
                // copied from intellij-rust, don't know what it's about
                let pat_ty = match named_item_kind {
                    Some(SyntaxKind::CONST) => ty,
                    _ => strip_references(ty, def_bm).0,
                };
                self.ctx.pat_types.insert(path_pat.into(), pat_ty);
            }
            ast::Pat::IdentPat(ident_pat) => {
                let named_item = self
                    .ctx
                    .resolve_ident_pat_cached(ident_pat.clone(), Some(ty.clone()))
                    .map(|it| it.value);
                let mut ident_pat_ty =
                    if matches!(named_item.map(|it| it.syntax().kind()), Some(SyntaxKind::VARIANT)) {
                        strip_references(ty, def_bm).0
                    } else {
                        apply_bm(ty, def_bm, self.ctx.msl)
                    };
                // special-case single element tuples
                if let Some(inner_ty) = ident_pat_ty.single_element_tuple_ty() {
                    ident_pat_ty = inner_ty;
                }
                self.ctx.pat_types.insert(ident_pat.into(), ident_pat_ty);
            }
            ast::Pat::StructPat(struct_pat) => {
                let (expected, pat_bm) = strip_references(ty.clone(), def_bm);
                self.ctx
                    .pat_types
                    .insert(struct_pat.clone().into(), expected.clone());

                let fields_owner = self.get_pat_fields_owner(struct_pat.path(), expected.clone());
                if let Some(fields_owner) = &fields_owner {
                    let (file_id, fields_owner) = fields_owner.unpack_ref();
                    if let StructOrEnum::Struct(struct_) = fields_owner.struct_or_enum() {
                        let pat_ty = self
                            .ctx
                            .instantiate_path(struct_pat.path().into(), struct_.in_file(file_id))
                            .into_ty_adt()?;
                        if !self.ctx.is_tys_compatible(expected.clone(), Ty::Adt(pat_ty)) {
                            self.ctx
                                .type_errors
                                .push(TypeError::invalid_unpacking(pat, ty.clone()));
                        }
                    }
                }

                let pat_fields = struct_pat.fields();
                let pat_field_tys = self.get_pat_field_tys(fields_owner, &pat_fields);
                let ty_adt_subst = expected
                    .into_ty_adt()
                    .map(|it| it.substitution)
                    .unwrap_or(empty_substitution());

                for (pat_field, (named_field_entry, ty)) in pat_fields.into_iter().zip(pat_field_tys) {
                    let pat_field_ty = ty.substitute(&ty_adt_subst);
                    match pat_field.field_kind() {
                        PatFieldKind::Full { pat: Some(pat), .. } => {
                            self.collect_pat_bindings(pat, pat_field_ty.clone(), pat_bm.clone());
                            self.ctx.pat_field_types.insert(pat_field, pat_field_ty);
                        }
                        PatFieldKind::Shorthand { ident_pat } => {
                            self.ctx.resolved_ident_pats.insert(ident_pat, named_field_entry);
                            self.ctx
                                .pat_field_types
                                .insert(pat_field, apply_bm(pat_field_ty, pat_bm.clone(), self.ctx.msl));
                        }
                        _ => (),
                    }
                }
            }
            ast::Pat::TupleStructPat(tuple_struct_pat) => {
                let (expected, pat_bm) = strip_references(ty.clone(), def_bm);
                self.ctx
                    .pat_types
                    .insert(tuple_struct_pat.clone().into(), expected.clone());

                let fields_owner = self.get_pat_fields_owner(tuple_struct_pat.path(), expected.clone());
                if fields_owner.is_none() {
                    for pat in tuple_struct_pat.fields() {
                        self.collect_pat_bindings(pat, Ty::Unknown, BindByValue);
                    }
                    return None;
                }
                let fields_owner = fields_owner.unwrap();

                let pats = tuple_struct_pat.fields().collect();

                let tuple_fields = fields_owner.map(|it| it.tuple_fields()).flatten();
                let tuple_field_types = tuple_fields
                    .into_iter()
                    .map(|field| {
                        self.ctx
                            .ty_lowering()
                            .lower_type_owner(field)
                            .unwrap_or(Ty::Unknown)
                    })
                    .collect::<Vec<_>>();
                let ty_adt_subst = expected
                    .into_ty_adt()
                    .map(|it| it.substitution)
                    .unwrap_or(empty_substitution());

                #[rustfmt::skip]
                self.infer_tuple_pat_fields(
                    pats,
                    tuple_field_types.len(),
                    pat_bm,
                    |indx| {
                        let field_type = tuple_field_types.get(indx).cloned().unwrap_or(Ty::Unknown);
                        field_type.substitute(&ty_adt_subst)
                    }
                );
            }
            ast::Pat::TuplePat(tuple_pat) => {
                let pats = tuple_pat.pats().collect::<Vec<_>>();
                if pats.len() == 1 && !matches!(ty, Ty::Tuple(_)) {
                    // let (a) = 1;
                    // let (a,) = 1;
                    let pat = pats.single_or_none().unwrap();
                    self.collect_pat_bindings(pat, ty, BindByValue);
                    return None;
                }

                if !self
                    .ctx
                    .is_tys_compatible(ty.clone(), Ty::Tuple(TyTuple::unknown(pats.len())))
                {
                    self.ctx
                        .type_errors
                        .push(TypeError::invalid_unpacking(pat, ty.clone()));
                }

                let inner_types = ty.into_ty_tuple().map(|it| it.types).unwrap_or_default();
                #[rustfmt::skip]
                self.infer_tuple_pat_fields(
                    pats,
                    inner_types.len(),
                    BindByValue,
                    |idx| {
                        inner_types.get(idx).cloned().unwrap_or(Ty::Unknown)
                    }
                );
            }
            ast::Pat::WildcardPat(wildcard_pat) => {
                self.ctx.pat_types.insert(wildcard_pat.into(), ty);
            }
            ast::Pat::RestPat(_) => (),
            ast::Pat::ParenPat(paren_pat) => {
                let inner_pat = paren_pat.pat()?;
                self.collect_pat_bindings(inner_pat, ty, def_bm);
            }
            ast::Pat::UnitPat(_) => (),
        };
        Some(())
    }

    fn get_pat_fields_owner(
        &mut self,
        struct_pat_path: ast::Path,
        expected_ty: Ty,
    ) -> Option<InFile<ast::FieldsOwner>> {
        let mut fields_owner = self
            .ctx
            .resolve_path_cached(struct_pat_path, Some(expected_ty.clone()))
            .and_then(|item| item.cast_into::<ast::FieldsOwner>());
        if fields_owner.is_none() {
            fields_owner = expected_ty
                .into_ty_adt()?
                .adt_item(self.ctx.db)?
                .cast_into::<ast::Struct>()
                .map(|it| it.map_into());
        }
        fields_owner
    }

    fn get_pat_field_tys(
        &mut self,
        fields_owner: Option<InFile<ast::FieldsOwner>>,
        pat_fields: &Vec<ast::StructPatField>,
    ) -> Vec<(Option<ScopeEntry>, Ty)> {
        if fields_owner.is_none() {
            return iter::repeat_n((None, Ty::Unknown), pat_fields.len()).collect();
        }
        let fields_owner = fields_owner.unwrap();
        let (item_file_id, fields_owner) = fields_owner.unpack();
        let ty_lowering = self.ctx.ty_lowering();
        let named_fields_map = fields_owner.named_fields_map();
        let mut tys = vec![];
        for pat_field in pat_fields {
            match pat_field
                .field_name()
                .and_then(|field_name| named_fields_map.get(&field_name))
            {
                Some(named_field) => {
                    let field_ty = ty_lowering
                        .lower_type_owner(named_field.to_owned().in_file(item_file_id))
                        .unwrap_or(Ty::Unknown);
                    tys.push((named_field.to_owned().in_file(item_file_id).to_entry(), field_ty));
                }
                None => {
                    tys.push((None, Ty::Unknown));
                }
            }
        }
        tys
    }

    fn infer_tuple_pat_fields(
        &mut self,
        pats: Vec<ast::Pat>,
        tuple_size: usize,
        bm: BindingMode,
        get_type: impl Fn(usize) -> Ty,
    ) {
        // In correct code, tuple or tuple struct patterns contain only one `..` pattern.
        // But it's pretty simple to support type inference for cases with multiple `..`
        // in patterns like `let (x, .., y, .., z) = tuple` by just ignoring all binding
        // between first and last `..` patterns
        let mut first_rest_indx = isize::MAX;
        let mut last_rest_indx = -1;
        for (indx, pat) in pats.iter().enumerate() {
            let indx = indx as isize;
            if matches!(pat, ast::Pat::RestPat(_)) {
                first_rest_indx = cmp::min(first_rest_indx, indx);
                last_rest_indx = cmp::max(last_rest_indx, indx);
            }
        }
        let pats_len = pats.len();
        for (indx, pat) in pats.into_iter().enumerate() {
            let indx = indx as isize;
            let pat_ty = if indx < first_rest_indx {
                get_type(indx as usize)
            } else if indx > last_rest_indx {
                get_type(tuple_size - (pats_len - (indx as usize)))
            } else {
                Ty::Unknown
            };
            self.collect_pat_bindings(pat, pat_ty, bm.clone());
        }
    }
}

pub fn anonymous_pat_ty_var(ty_var_index: &TyVarIndex, pat: &ast::Pat) -> Ty {
    match pat {
        ast::Pat::IdentPat(_) => Ty::new_ty_var(ty_var_index),
        ast::Pat::TuplePat(tuple_pat) => {
            let pat_types = tuple_pat.pats().map(|_| Ty::new_ty_var(ty_var_index)).collect();
            Ty::Tuple(TyTuple::new(pat_types))
        }
        _ => Ty::Unknown,
    }
}

#[derive(Debug, Clone)]
pub enum BindingMode {
    BindByValue,
    BindByReference { mutability: Mutability },
}

fn apply_bm(ty: Ty, def_bm: BindingMode, _msl: bool) -> Ty {
    match def_bm {
        BindByReference { mutability } => Ty::new_reference(ty, mutability),
        BindByValue => ty,
    }
}

fn strip_references(ty: Ty, def_bm: BindingMode) -> (Ty, BindingMode) {
    let mut bm = def_bm;
    let mut ty = ty;
    while let Ty::Reference(ty_ref) = &ty {
        bm = match bm.clone() {
            BindByReference { mutability: old_mut } => {
                let new_mutability = if old_mut == Mutability::Immutable {
                    Mutability::Immutable
                } else {
                    ty_ref.mutability.to_owned()
                };
                BindByReference { mutability: new_mutability }
            }
            BindByValue => BindByReference {
                mutability: ty_ref.mutability.to_owned(),
            },
        };
        ty = ty_ref.referenced();
    }
    (ty, bm)
}
