use crate::loc::{SyntaxLoc, SyntaxLocFileExt, SyntaxLocNodeExt};
use crate::types::fold::{TypeFoldable, TypeFolder, TypeVisitor};
use crate::types::inference::InferenceCtx;
use crate::types::ty::Ty;
use crate::types::ty::adt::TyAdt;
use crate::types::ty::reference::TyReference;
use crate::types::ty::tuple::TyTuple;
use crate::types::ty::ty_callable::TyCallable;
use crate::types::ty::ty_var::{TyInfer, TyIntVar, TyVar};
use std::cell::RefCell;
use std::iter::zip;
use syntax::files::{InFile, InFileExt};
use syntax::{SyntaxNodeOrToken, ast};

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
            Err(error_tys) => {
                self.report_type_mismatch(
                    error_tys,
                    node_or_token.in_file(self.file_id),
                    actual,
                    expected,
                );
                false
            }
        }
    }

    pub fn combine_types(&mut self, left_ty: Ty, right_ty: Ty) -> CombineResult {
        let left_ty = left_ty.refine_for_specs(self.msl);
        let right_ty = right_ty.refine_for_specs(self.msl);

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
                Err(MismatchErrorTypes::new(left_ty, right_ty))
            }
            (Ty::Seq(ty_seq1), Ty::Seq(ty_seq2)) => self.combine_types(ty_seq1.item(), ty_seq2.item()),
            (Ty::Reference(from_ref), Ty::Reference(to_ref)) => self.combine_ty_refs(from_ref, to_ref),
            (Ty::Callable(ty_call1), Ty::Callable(ty_call2)) => {
                self.combine_ty_callables(ty_call1, ty_call2)
            }

            (Ty::Adt(ty_adt1), Ty::Adt(ty_adt2)) => self.combine_ty_adts(ty_adt1, ty_adt2),
            (Ty::Tuple(ty_tuple1), Ty::Tuple(ty_tuple2)) => self.combine_ty_tuples(ty_tuple1, ty_tuple2),

            _ => Err(MismatchErrorTypes::new(left_ty, right_ty)),
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
            _ => {
                return Err(MismatchErrorTypes::new(Ty::Infer(TyInfer::IntVar(int_var)), ty));
            }
        }
        Ok(())
    }

    fn combine_ty_refs(&mut self, from_ref: &TyReference, to_ref: &TyReference) -> CombineResult {
        let is_mut_compat = from_ref.is_mut() || !to_ref.is_mut();
        if !is_mut_compat {
            // combine inner types ignoring any errors, to have better type errors and later inference,
            // incompat error will still be reported
            let _ = self.combine_types(from_ref.referenced(), to_ref.referenced());
            return Err(MismatchErrorTypes::new(
                from_ref.to_owned().into(),
                to_ref.to_owned().into(),
            ));
        }
        self.combine_types(from_ref.referenced(), to_ref.referenced())
    }

    fn combine_ty_callables(&mut self, ty1: &TyCallable, ty2: &TyCallable) -> CombineResult {
        // todo: check param types size
        self.combine_ty_pairs(ty1.clone().param_types, ty2.clone().param_types)?;
        // todo: resolve variables?
        self.combine_types(ty1.ret_type().unwrap_all_refs(), ty2.ret_type().unwrap_all_refs())
    }

    fn combine_ty_adts(&mut self, ty1: &TyAdt, ty2: &TyAdt) -> CombineResult {
        if ty1.adt_item_loc != ty2.adt_item_loc {
            return Err(MismatchErrorTypes::new(
                Ty::Adt(ty1.to_owned()),
                Ty::Adt(ty2.to_owned()),
            ));
        }
        self.combine_ty_pairs(ty1.clone().type_args, ty2.clone().type_args)
    }

    fn combine_ty_tuples(&mut self, ty1: &TyTuple, ty2: &TyTuple) -> CombineResult {
        if ty1.types.len() != ty2.types.len() {
            return Err(MismatchErrorTypes::new(
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
                            Ty::new_reference(left_ty_ref.referenced().unwrap_all_refs(), min_mut)
                        }
                        _ => left_ty,
                    }
                } else {
                    Ty::Unknown
                }
            }
        }
    }

    fn report_type_mismatch(
        &mut self,
        _mismatch_error_tys: MismatchErrorTypes,
        node_or_token: InFile<SyntaxNodeOrToken>,
        actual: Ty,
        expected: Ty,
    ) {
        let type_error = TypeError::type_mismatch(node_or_token, expected, actual);
        self.type_errors.push(type_error);
    }
}

pub type CombineResult = Result<(), MismatchErrorTypes>;

#[derive(Debug)]
pub struct MismatchErrorTypes {
    ty1: Ty,
    ty2: Ty,
}

impl MismatchErrorTypes {
    pub fn new(ty1: Ty, ty2: Ty) -> Self {
        MismatchErrorTypes { ty1, ty2 }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TypeError {
    TypeMismatch {
        loc: SyntaxLoc,
        expected_ty: Ty,
        actual_ty: Ty,
    },
    UnsupportedOp {
        loc: SyntaxLoc,
        ty: Ty,
        op: String,
    },
    WrongArgumentsToBinExpr {
        loc: SyntaxLoc,
        left_ty: Ty,
        right_ty: Ty,
        op: String,
    },
    InvalidUnpacking {
        loc: SyntaxLoc,
        assigned_ty: Ty,
    },
}

impl TypeError {
    pub fn loc(&self) -> SyntaxLoc {
        match self {
            TypeError::TypeMismatch { loc, .. } => loc.clone(),
            TypeError::UnsupportedOp { loc, .. } => loc.clone(),
            TypeError::WrongArgumentsToBinExpr { loc, .. } => loc.clone(),
            TypeError::InvalidUnpacking { loc, .. } => loc.clone(),
        }
    }
    pub fn type_mismatch(
        node_or_token: InFile<SyntaxNodeOrToken>,
        expected_ty: Ty,
        actual_ty: Ty,
    ) -> Self {
        let (file_id, node_or_token) = node_or_token.unpack();
        TypeError::TypeMismatch {
            loc: SyntaxLoc::from_node_or_token(file_id, node_or_token),
            expected_ty,
            actual_ty,
        }
    }

    pub fn unsupported_op(expr: InFile<ast::Expr>, ty: Ty, op: ast::BinaryOp) -> Self {
        TypeError::UnsupportedOp {
            loc: expr.loc(),
            ty,
            op: op.to_string(),
        }
    }

    pub fn wrong_arguments_to_bin_expr(
        expr: InFile<ast::BinExpr>,
        left_ty: Ty,
        right_ty: Ty,
        op: ast::BinaryOp,
    ) -> Self {
        TypeError::WrongArgumentsToBinExpr {
            loc: expr.loc(),
            left_ty,
            right_ty,
            op: op.to_string(),
        }
    }

    pub fn invalid_unpacking(pat: InFile<ast::Pat>, assigned_ty: Ty) -> Self {
        TypeError::InvalidUnpacking { loc: pat.loc(), assigned_ty }
    }
}

impl TypeFoldable<TypeError> for TypeError {
    fn deep_fold_with(self, folder: impl TypeFolder) -> TypeError {
        match self {
            TypeError::TypeMismatch { loc, expected_ty, actual_ty } => TypeError::TypeMismatch {
                loc,
                expected_ty: expected_ty.fold_with(folder.clone()),
                actual_ty: actual_ty.fold_with(folder),
            },
            TypeError::UnsupportedOp { loc, ty, op } => TypeError::UnsupportedOp {
                loc,
                ty: ty.fold_with(folder),
                op,
            },
            TypeError::WrongArgumentsToBinExpr { loc, left_ty, right_ty, op } => {
                TypeError::WrongArgumentsToBinExpr {
                    loc,
                    left_ty: left_ty.fold_with(folder.clone()),
                    right_ty: right_ty.fold_with(folder),
                    op,
                }
            }
            TypeError::InvalidUnpacking { loc, assigned_ty } => TypeError::InvalidUnpacking {
                loc,
                assigned_ty: assigned_ty.fold_with(folder),
            },
        }
    }

    fn deep_visit_with(&self, _visitor: impl TypeVisitor) -> bool {
        true
    }
}
