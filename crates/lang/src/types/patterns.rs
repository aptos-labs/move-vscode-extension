use crate::nameres::scope::{ScopeEntry, ScopeEntryExt};
use crate::types::inference::InferenceCtx;
use crate::types::inference::ast_walker::TypeAstWalker;
use crate::types::patterns::BindingMode::{BindByReference, BindByValue};
use crate::types::substitution::{ApplySubstitution, empty_substitution};
use crate::types::ty::Ty;
use crate::types::ty::reference::{Mutability, TyReference};
use crate::types::ty::tuple::TyTuple;
use parser::SyntaxKind;
use std::iter;
use syntax::ast::FieldsOwner;
use syntax::ast::node_ext::struct_pat_field::PatFieldKind;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, ast};

impl TypeAstWalker<'_, '_> {
    pub fn collect_pat_bindings(&mut self, pat: ast::Pat, ty: Ty, def_bm: BindingMode) {
        match pat {
            ast::Pat::PathPat(path_pat) => {
                let named_item = self.ctx.resolve_path_cached(path_pat.path(), None);
                let named_item_kind = named_item.map(|it| it.value.syntax().kind());
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
                let ident_pat_ty =
                    if matches!(named_item.map(|it| it.syntax().kind()), Some(SyntaxKind::VARIANT)) {
                        strip_references(ty, def_bm).0
                    } else {
                        apply_bm(ty, def_bm, self.ctx.msl)
                    };
                self.ctx.pat_types.insert(ident_pat.into(), ident_pat_ty);
            }
            ast::Pat::StructPat(struct_pat) => {
                let (expected, pat_bm) = strip_references(ty.clone(), def_bm);
                self.ctx
                    .pat_types
                    .insert(struct_pat.clone().into(), expected.clone());

                let mut fields_owner = self
                    .ctx
                    .resolve_path_cached(struct_pat.path(), Some(expected.clone()))
                    .and_then(|item| item.cast::<ast::AnyFieldsOwner>());
                if fields_owner.is_none() {
                    fields_owner = expected.into_ty_adt().and_then(|it| {
                        it.adt_item(self.ctx.db)
                            .and_then(|it| it.cast::<ast::Struct>())
                            .map(|it| it.in_file_into())
                    });
                }
                // todo: invalid unpacking

                let pat_fields = struct_pat.fields();
                let pat_field_tys = self.get_pat_field_tys(fields_owner, &pat_fields);
                let ty_adt_subst = ty
                    .into_ty_adt()
                    .map(|it| it.substitution)
                    .unwrap_or(empty_substitution());

                for (pat_field, (named_field_entry, ty)) in pat_fields.into_iter().zip(pat_field_tys) {
                    let pat_field_ty = ty.substitute(&ty_adt_subst);
                    match pat_field.kind() {
                        PatFieldKind::Full { pat, .. } => {
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
            ast::Pat::WildcardPat(wildcard_pat) => {
                self.ctx.pat_types.insert(wildcard_pat.into(), ty);
            }
            _ => (),
        }
    }

    fn get_pat_field_tys(
        &mut self,
        fields_owner: Option<InFile<ast::AnyFieldsOwner>>,
        pat_fields: &Vec<ast::StructPatField>,
    ) -> Vec<(Option<ScopeEntry>, Ty)> {
        if fields_owner.is_none() {
            return iter::repeat_n((None, Ty::Unknown), pat_fields.len()).collect();
        }
        let fields_owner = fields_owner.unwrap();
        let (item_file_id, fields_owner) = fields_owner.unpack();
        let named_fields_map = fields_owner.named_fields_map();
        let mut tys = vec![];
        for pat_field in pat_fields {
            let Some(named_field) = pat_field
                .field_name()
                .and_then(|field_name| named_fields_map.get(&field_name))
            else {
                tys.push((None, Ty::Unknown));
                continue;
            };
            let field_ty = self
                .ctx
                .ty_lowering()
                .lower_field(named_field.to_owned().in_file(item_file_id))
                .unwrap_or(Ty::Unknown);
            tys.push((named_field.to_owned().in_file(item_file_id).to_entry(), field_ty));
        }
        tys
    }
}

pub fn anonymous_pat_ty_var(ctx: &mut InferenceCtx, pat: &ast::Pat) -> Ty {
    match pat {
        ast::Pat::IdentPat(_) => Ty::new_ty_var(ctx),
        ast::Pat::TuplePat(tuple_pat) => {
            let pat_types = tuple_pat.pats().map(|_| Ty::new_ty_var(ctx)).collect();
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
                BindByReference {
                    mutability: new_mutability,
                }
            }
            BindByValue => BindByReference {
                mutability: ty_ref.mutability.to_owned(),
            },
        };
        ty = ty_ref.referenced();
    }
    (ty, bm)
}
