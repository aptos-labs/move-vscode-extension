use crate::types::ty::Ty;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

#[derive(Debug, Clone)]
pub enum TableValue<Var> {
    Var(Var),
    Value(Option<Ty>),
}

#[derive(Debug, Clone)]
pub struct UnificationTable<Var: Clone + Eq + Hash + Debug> {
    mapping: HashMap<Var, TableValue<Var>>,
    snapshots: Vec<HashMap<Var, TableValue<Var>>>,
}

impl<Var: Clone + Eq + Hash + Debug> UnificationTable<Var> {
    pub fn new() -> Self {
        UnificationTable {
            mapping: HashMap::new(),
            snapshots: vec![],
        }
    }

    pub fn unify_var_var(&mut self, left_var: &Var, right_var: &Var) {
        let left_root_var = self.resolve_to_root_var(left_var);
        let right_root_var = self.resolve_to_root_var(right_var);
        if left_root_var == right_root_var {
            // already unified
            return;
        }

        let left_value_ty = self.resolve_to_ty_value(&left_root_var);
        let right_value_ty = self.resolve_to_ty_value(&right_root_var);

        let new_value_ty = match (&left_value_ty, &right_value_ty) {
            (Some(left_ty), Some(right_ty)) => {
                if *left_ty != *right_ty {
                    panic!("unification error: if both vars are unified, their ty's must be the same")
                }
                Some(left_ty.to_owned())
            }
            _ => left_value_ty.or(right_value_ty),
        };

        self.mapping
            .insert(left_root_var, TableValue::Var(right_root_var.clone()));
        self.mapping
            .insert(right_root_var, TableValue::Value(new_value_ty));
    }

    pub fn unify_var_value(&mut self, ty_var: &Var, ty: Ty) {
        let old_value_ty = self.resolve_to_ty_value(ty_var);
        if let Some(old_value_ty) = old_value_ty {
            // if already unified, value must be the same
            if old_value_ty != ty {
                panic!("unification error, {old_value_ty:?} != {ty:?}")
            }
            return;
        }
        let root_var = self.resolve_to_root_var(ty_var);
        self.mapping.insert(root_var, TableValue::Value(Some(ty)));
    }

    pub fn resolve_to_root_var(&self, var: &Var) -> Var {
        let mut var = var;
        while let Some(TableValue::Var(root_var)) = self.mapping.get(var) {
            var = root_var;
        }
        var.to_owned()
    }

    pub fn resolve_to_ty_value(&self, var: &Var) -> Option<Ty> {
        let root_var = self.resolve_to_root_var(var);
        self.mapping.get(&root_var).and_then(|t_value| match t_value {
            TableValue::Value(ty) => ty.to_owned(),
            TableValue::Var(_) => None,
        })
    }

    pub fn snapshot(&mut self) {
        self.snapshots.push(self.mapping.clone());
    }

    pub fn rollback(&mut self) {
        let last_snapshot = self.snapshots.pop().expect("inconsistent snapshot rollback");
        self.mapping = last_snapshot;
    }
}
