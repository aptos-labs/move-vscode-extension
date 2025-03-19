use crate::types::fold::TypeFoldable;
use crate::types::ty::ty_var::{TyVar, TyVarKind};
use crate::types::ty::type_param::TyTypeParameter;
use crate::types::ty::{Ty, TypeFolder};
use std::collections::HashMap;

#[derive(Debug)]
pub enum TableValue {
    Var(TyVar),
    Value(Ty),
}

#[derive(Debug)]
pub struct UnificationTable {
    mapping: HashMap<TyVar, TableValue>,
}

impl UnificationTable {
    pub fn new() -> Self {
        UnificationTable {
            mapping: HashMap::new(),
        }
    }

    pub fn unify_var_value(&mut self, ty_var: TyVar, ty: Ty) {
        // resolve `ty_var` with mapping, and if it's in the `mapping`, then it's an error
        self.mapping.insert(ty_var, TableValue::Value(ty));
    }

    pub fn resolve_ty_var(&self, ty_var: &TyVar) -> Option<Ty> {
        self.mapping
            .get(ty_var)
            .and_then(|table_value| match table_value {
                TableValue::Value(ty) => Some(ty.clone()),
                TableValue::Var(_) => None,
            })
    }
}

#[derive(Debug, Clone)]
pub struct TyVarResolver<'a> {
    uni_table: &'a UnificationTable,
}

impl<'a> TyVarResolver<'a> {
    pub fn new(unification_table: &'a UnificationTable) -> Self {
        TyVarResolver {
            uni_table: unification_table,
        }
    }
}

impl TypeFolder for TyVarResolver<'_> {
    fn fold_ty(&self, t: Ty) -> Ty {
        match t {
            Ty::Var(ref ty_var) => self.uni_table.resolve_ty_var(ty_var).unwrap_or(t),
            _ => t.deep_fold_with(self.to_owned()),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Fallback {
    TyUnknown,
    Origin,
}

#[derive(Debug, Clone)]
pub struct FullTyVarResolver<'a> {
    uni_table: &'a UnificationTable,
    fallback: Fallback,
}

impl<'a> FullTyVarResolver<'a> {
    pub fn new(unification_table: &'a UnificationTable, fallback: Fallback) -> Self {
        FullTyVarResolver {
            uni_table: unification_table,
            fallback,
        }
    }
}

impl TypeFolder for FullTyVarResolver<'_> {
    fn fold_ty(&self, t: Ty) -> Ty {
        match t {
            Ty::Var(ref ty_var) => {
                let resolved_ty_var = self.uni_table.resolve_ty_var(ty_var);
                match resolved_ty_var {
                    Some(ty) => ty,
                    None => match (self.fallback, &ty_var.kind) {
                        (Fallback::Origin, TyVarKind::WithOrigin { origin_loc }) => {
                            Ty::TypeParam(TyTypeParameter::from_loc(origin_loc.to_owned()))
                        }
                        _ => Ty::Unknown,
                    },
                }
            }
            _ => t.deep_fold_with(self.to_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::fold::TypeFoldable;
    use crate::types::ty::Ty;
    use crate::types::unification::{TyVar, TyVarResolver, UnificationTable};

    #[test]
    fn test_resolve_ty_var_after_unification() {
        let mut unification_table = UnificationTable::new();

        let v_arg = TyVar::new_anonymous(0);
        unification_table.unify_var_value(v_arg.clone(), Ty::Bool);

        let v = Ty::Vector(Box::new(Ty::Var(v_arg)));
        let resolved_v = v.deep_fold_with(TyVarResolver::new(&unification_table));
        dbg!(resolved_v);
    }
}
