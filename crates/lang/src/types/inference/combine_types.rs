use crate::types::inference::InferenceCtx;
use crate::types::ty::Ty;
use crate::types::ty::adt::TyAdt;
use crate::types::ty::reference::TyReference;
use crate::types::ty::tuple::TyTuple;
use crate::types::ty::ty_callable::TyCallable;
use crate::types::ty::ty_var::{TyInfer, TyIntVar, TyVar};
use std::cell::RefCell;
use std::iter::zip;
use syntax::SyntaxNodeOrToken;

impl InferenceCtx<'_> {
    #[allow(clippy::wrong_self_convention)]
    pub fn is_tys_compatible(&mut self, ty: Ty, into_ty: Ty) -> bool {
        self.freeze(|ctx| ctx.combine_types(ty, into_ty).is_ok())
    }

    pub fn coerce_types(&mut self, node_or_token: SyntaxNodeOrToken, actual: Ty, expected: Ty) -> bool {
        let actual = self.resolve_ty_vars_if_possible(actual);
        let expected = self.resolve_ty_vars_if_possible(expected);
        if actual == expected {
            return true;
        }
        let combined = self.combine_types(actual.clone(), expected.clone());
        match combined {
            Ok(()) => true,
            Err(type_error) => {
                // todo: report type error at `node`
                self.report_type_error(type_error, node_or_token, actual, expected);
                false
            }
        }
    }

    pub fn combine_types(&mut self, left_ty: Ty, right_ty: Ty) -> CombineResult {
        let left_ty = self.resolve_ty_infer_shallow(left_ty);
        let right_ty = self.resolve_ty_infer_shallow(right_ty);

        match (left_ty, right_ty) {
            (Ty::Infer(TyInfer::Var(ty_var)), right_ty) => self.unify_ty_var(&ty_var, right_ty),
            (left_ty, Ty::Infer(TyInfer::Var(ty_var))) => self.unify_ty_var(&ty_var, left_ty),

            (Ty::Infer(TyInfer::IntVar(int_var)), right_ty) => self.unify_int_var(int_var, right_ty),
            (left_ty, Ty::Infer(TyInfer::IntVar(int_var))) => self.unify_int_var(int_var, left_ty),

            (left_ty, right_ty) => self.combine_no_vars(left_ty, right_ty),
        }
    }

    fn combine_no_vars(&mut self, left_ty: Ty, right_ty: Ty) -> CombineResult {
        // assign Ty::Unknown to all inner `TyVar`s if other type is unknown
        if matches!(left_ty, Ty::Unknown) || matches!(right_ty, Ty::Unknown) {
            self.unify_ty_vars_with_unknown(vec![left_ty, right_ty]);
            return Ok(());
        }
        // if never type is involved, do not perform comparison
        if matches!(left_ty, Ty::Never) || matches!(right_ty, Ty::Never) {
            return Ok(());
        }
        // if type are exactly equal, then they're compatible
        if left_ty == right_ty {
            return Ok(());
        }

        match (&left_ty, &right_ty) {
            (Ty::Integer(kind1), Ty::Integer(kind2)) => {
                if kind1.is_default() || kind2.is_default() {
                    return Ok(());
                }
                Err(TypeError::new(left_ty, right_ty))
            }
            (Ty::Seq(ty_seq1), Ty::Seq(ty_seq2)) => self.combine_types(ty_seq1.item(), ty_seq2.item()),
            (Ty::Reference(from_ref), Ty::Reference(to_ref)) => self.combine_ty_refs(from_ref, to_ref),
            (Ty::Callable(ty_call1), Ty::Callable(ty_call2)) => {
                self.combine_ty_callables(ty_call1, ty_call2)
            }

            (Ty::Adt(ty_adt1), Ty::Adt(ty_adt2)) => self.combine_ty_adts(ty_adt1, ty_adt2),
            (Ty::Tuple(ty_tuple1), Ty::Tuple(ty_tuple2)) => self.combine_ty_tuples(ty_tuple1, ty_tuple2),

            _ => Err(TypeError::new(left_ty, right_ty)),
        }
    }

    fn unify_ty_vars_with_unknown(&mut self, tys: Vec<Ty>) {
        let ty_vars = RefCell::new(vec![]);
        for ty in tys {
            ty.deep_visit_ty_infers(|ty_infer| {
                if let TyInfer::Var(ty_var) = ty_infer {
                    ty_vars.borrow_mut().push(ty_var.clone());
                };
                false
            });
        }
        for ty_var in ty_vars.into_inner() {
            let _ = self.unify_ty_var(&ty_var, Ty::Unknown);
        }
    }

    fn unify_ty_var(&mut self, var: &TyVar, ty: Ty) -> CombineResult {
        match ty {
            Ty::Infer(TyInfer::Var(ty_var)) => self.var_table.unify_var_var(var, &ty_var),
            _ => {
                let root_ty_var = self.var_table.resolve_to_root_var(var);
                if self.ty_contains_ty_var(&ty, &root_ty_var) {
                    // "E0308 cyclic type of infinite size"
                    self.var_table.unify_var_value(&root_ty_var, Ty::Unknown);
                    return Ok(());
                }
                self.var_table.unify_var_value(&root_ty_var, ty);
            }
        };
        Ok(())
    }

    fn ty_contains_ty_var(&self, ty: &Ty, ty_var: &TyVar) -> bool {
        ty.deep_visit_ty_infers(|ty_infer| match ty_infer {
            TyInfer::Var(inner_ty_var) => &self.var_table.resolve_to_root_var(&inner_ty_var) == ty_var,
            _ => false,
        })
    }

    pub(crate) fn unify_int_var(&mut self, int_var: TyIntVar, ty: Ty) -> CombineResult {
        match ty {
            Ty::Infer(TyInfer::IntVar(ty_int_var)) => {
                self.int_table.unify_var_var(&int_var, &ty_int_var)
            }
            Ty::Integer(_) => self.int_table.unify_var_value(&int_var, ty),
            Ty::Unknown => {
                // do nothing, unknown should not influence IntVar
            }
            _ => return Err(TypeError::new(Ty::Infer(TyInfer::IntVar(int_var)), ty)),
        }
        Ok(())
    }

    fn combine_ty_refs(&mut self, from_ref: &TyReference, to_ref: &TyReference) -> CombineResult {
        let is_mut_compat = from_ref.is_mut() || !to_ref.is_mut();
        if !is_mut_compat {
            return Err(TypeError::new(
                Ty::Reference(from_ref.to_owned()),
                Ty::Reference(to_ref.to_owned()),
            ));
        }
        self.combine_types(from_ref.referenced().to_owned(), to_ref.referenced().to_owned())
    }

    fn combine_ty_callables(&mut self, ty1: &TyCallable, ty2: &TyCallable) -> CombineResult {
        // todo: check param types size
        self.combine_ty_pairs(ty1.clone().param_types, ty2.clone().param_types)?;
        // todo: resolve variables?
        self.combine_types(
            ty1.ret_type.deref_all().to_owned(),
            ty2.ret_type.deref_all().to_owned(),
        )
    }

    fn combine_ty_adts(&mut self, ty1: &TyAdt, ty2: &TyAdt) -> CombineResult {
        if ty1.adt_item != ty2.adt_item {
            return Err(TypeError::new(Ty::Adt(ty1.to_owned()), Ty::Adt(ty2.to_owned())));
        }
        self.combine_ty_pairs(ty1.clone().type_args, ty2.clone().type_args)
    }

    fn combine_ty_tuples(&mut self, ty1: &TyTuple, ty2: &TyTuple) -> CombineResult {
        if ty1.types.len() != ty2.types.len() {
            return Err(TypeError::new(
                Ty::Tuple(ty1.to_owned()),
                Ty::Tuple(ty2.to_owned()),
            ));
        }
        self.combine_ty_pairs(ty1.clone().types, ty2.clone().types)
    }

    fn combine_ty_pairs(&mut self, left_tys: Vec<Ty>, right_tys: Vec<Ty>) -> CombineResult {
        let mut can_unify = Ok(());
        let pairs = zip(left_tys.into_iter(), right_tys.into_iter());
        for (ty1, ty2) in pairs {
            can_unify = can_unify.and(self.combine_types(ty1, ty2));
        }
        can_unify
    }

    pub fn intersect_all_types(&mut self, types: Vec<Ty>) -> Ty {
        types
            .into_iter()
            .reduce(|acc, ty| self.intersect_types(acc, ty))
            .unwrap_or(Ty::Unknown)
    }

    fn intersect_types(&mut self, left_ty: Ty, right_ty: Ty) -> Ty {
        match (&left_ty, &right_ty) {
            (Ty::Never, _) => right_ty,
            (_, Ty::Never) => left_ty,
            (Ty::Unknown, _) => right_ty, // even if Ty::Unknown too
            _ => {
                let is_ok = self.combine_types(left_ty.clone(), right_ty.clone()).is_ok()
                    || self.combine_types(right_ty.clone(), left_ty.clone()).is_ok();
                if is_ok {
                    match (left_ty.clone(), right_ty) {
                        (Ty::Reference(left_ty_ref), Ty::Reference(right_ty_ref)) => {
                            let min_mut = left_ty_ref.mutability.intersect(right_ty_ref.mutability);
                            Ty::Reference(TyReference::new(
                                left_ty_ref.referenced.deref_all().to_owned(),
                                min_mut,
                            ))
                        }
                        _ => left_ty,
                    }
                } else {
                    Ty::Unknown
                }
            }
        }
    }

    fn report_type_error(
        &mut self,
        _type_error: TypeError,
        _node_or_token: SyntaxNodeOrToken,
        _actual: Ty,
        _expected: Ty,
    ) {
        // todo: report type error at `node`
    }
}

pub type CombineResult = Result<(), TypeError>;
pub struct TypeError {
    ty1: Ty,
    ty2: Ty,
}
impl TypeError {
    pub fn new(ty1: Ty, ty2: Ty) -> Self {
        TypeError { ty1, ty2 }
    }
}
