use crate::types::fold::{TypeFoldable, TypeFolder, TypeVisitor};
use crate::types::inference::InferenceCtx;
use crate::types::substitution::ApplySubstitution;
use crate::types::ty::Ty;
use crate::types::ty::adt::TyAdt;
use crate::types::ty::reference::TyReference;
use crate::types::ty::tuple::TyTuple;
use crate::types::ty::ty_callable::TyCallable;
use crate::types::ty::ty_var::{TyInfer, TyIntVar, TyVar};
use std::cell::RefCell;
use std::iter::zip;
use syntax::{AstNode, SyntaxKind, SyntaxNodeOrToken, TextRange, ast};

impl InferenceCtx<'_> {
    #[allow(clippy::wrong_self_convention)]
    pub fn is_tys_compatible(&mut self, ty: Ty, into_ty: Ty) -> bool {
        // zero element type is unit expr and compatible with anything
        if matches!(&into_ty, Ty::Tuple(ty_tuple) if ty_tuple.types.len() == 0) {
            return true;
        }
        self.freeze(|ctx| ctx.combine_types(ty, into_ty).is_ok())
    }

    pub fn coerce_types(&mut self, node_or_token: SyntaxNodeOrToken, actual: Ty, expected: Ty) -> bool {
        let actual = self.resolve_ty_vars_if_possible(actual);
        let expected = self.resolve_ty_vars_if_possible(expected);
        if actual == expected {
            return true;
        }
        let combined = self.combine_types(expected.clone(), actual.clone());
        match combined {
            Ok(()) => true,
            Err(error_tys) => {
                self.report_type_mismatch(error_tys, node_or_token, actual, expected);
                false
            }
        }
    }

    pub fn combine_types(&mut self, expected_ty: Ty, actual_ty: Ty) -> CombineResult {
        let expected_ty = expected_ty.refine_for_specs(self.msl);

        let mut actual_ty = actual_ty.refine_for_specs(self.msl);
        if let Some(inner_tuple_ty) = actual_ty.single_element_tuple_ty() {
            if !matches!(expected_ty, Ty::Tuple(_)) {
                actual_ty = inner_tuple_ty;
            }
        }

        let expected_ty = self.resolve_ty_infer_shallow(expected_ty);
        let actual_ty = self.resolve_ty_infer_shallow(actual_ty);

        match (expected_ty, actual_ty) {
            (Ty::Infer(TyInfer::Var(ty_var)), right_ty) => self.unify_ty_var(&ty_var, right_ty),
            (left_ty, Ty::Infer(TyInfer::Var(ty_var))) => self.unify_ty_var(&ty_var, left_ty),

            (Ty::Infer(TyInfer::IntVar(int_var)), right_ty) => self.unify_int_var(int_var, right_ty),
            (left_ty, Ty::Infer(TyInfer::IntVar(int_var))) => self.unify_int_var(int_var, left_ty),

            (expected_ty, actual_ty) => self.combine_no_vars(expected_ty, actual_ty),
        }
    }

    fn combine_no_vars(&mut self, expected_ty: Ty, actual_ty: Ty) -> CombineResult {
        // assign Ty::Unknown to all inner `TyVar`s if other type is unknown
        if matches!(expected_ty, Ty::Unknown) || matches!(actual_ty, Ty::Unknown) {
            self.unify_ty_vars_with_unknown(vec![expected_ty, actual_ty]);
            return Ok(());
        }
        // if never type is involved, do not perform comparison
        if matches!(expected_ty, Ty::Never) || matches!(actual_ty, Ty::Never) {
            return Ok(());
        }
        // if type are exactly equal, then they're compatible
        if expected_ty == actual_ty {
            return Ok(());
        }

        match (&expected_ty, &actual_ty) {
            (Ty::Integer(expected_kind), Ty::Integer(actual_kind)) => {
                if expected_kind.is_default() || actual_kind.is_default() {
                    return Ok(());
                }
                Err(MismatchErrorTypes::new(expected_ty, actual_ty))
            }
            (Ty::Seq(expected_seq_ty), Ty::Seq(actual_seq_ty)) => {
                self.combine_types(expected_seq_ty.item(), actual_seq_ty.item())
            }
            (Ty::Reference(expected_ref), Ty::Reference(actual_ref)) => {
                self.combine_ty_refs(expected_ref, actual_ref)
            }
            // new type struct
            (Ty::Adt(expected_ty_adt), Ty::Callable(actual_callable_ty)) => {
                if let Some(combine_result) =
                    self.combine_new_type_struct_with_lambda(expected_ty_adt, actual_callable_ty)
                {
                    return combine_result;
                }
                Err(MismatchErrorTypes::new(expected_ty, actual_ty))
            }
            (Ty::Callable(expected_call_ty), Ty::Callable(actual_call_ty)) => {
                self.combine_ty_callables(expected_call_ty, actual_call_ty)
            }

            (Ty::Adt(ty_adt1), Ty::Adt(ty_adt2)) => self.combine_ty_adts(ty_adt1, ty_adt2),
            (Ty::Tuple(ty_tuple1), Ty::Tuple(ty_tuple2)) => self.combine_ty_tuples(ty_tuple1, ty_tuple2),

            _ => Err(MismatchErrorTypes::new(expected_ty, actual_ty)),
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

    fn combine_ty_refs(
        &mut self,
        expected_ref: &TyReference,
        actual_ref: &TyReference,
    ) -> CombineResult {
        let is_mut_compat = !expected_ref.is_mut() || actual_ref.is_mut();
        if !is_mut_compat {
            // combine inner types ignoring any errors, to have better type errors and later inference,
            // incompat error will still be reported
            let _ = self.combine_types(expected_ref.referenced(), actual_ref.referenced());
            return Err(MismatchErrorTypes::new(
                expected_ref.to_owned().into(),
                actual_ref.to_owned().into(),
            ));
        }
        self.combine_types(expected_ref.referenced(), actual_ref.referenced())
    }

    fn combine_ty_callables(
        &mut self,
        expected_call_ty: &TyCallable,
        actual_call_ty: &TyCallable,
    ) -> CombineResult {
        // todo: check param types size
        self.combine_ty_pairs(
            expected_call_ty.clone().param_types,
            actual_call_ty.clone().param_types,
        )?;
        // todo: resolve variables?
        self.combine_types(
            expected_call_ty.ret_type().unwrap_all_refs(),
            actual_call_ty.ret_type().unwrap_all_refs(),
        )
    }

    fn combine_new_type_struct_with_lambda(
        &mut self,
        expected_ty_adt: &TyAdt,
        actual_callable_ty: &TyCallable,
    ) -> Option<CombineResult> {
        let struct_inner_lambda_type = expected_ty_adt
            .adt_item(self.db)?
            .and_then(|item| item.struct_()?.wrapped_lambda_type())?;
        let expected_lambda_ty = self
            .ty_lowering()
            .lower_type(struct_inner_lambda_type.map_into())
            .into_ty_callable()?
            .substitute(&expected_ty_adt.substitution);
        Some(self.combine_ty_callables(&expected_lambda_ty, actual_callable_ty))
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
        // needs to handle TyUnknown properly in the combine_types() later
        let resolved_types = types
            .into_iter()
            .map(|it| self.resolve_ty_vars_if_possible(it))
            .collect::<Vec<_>>();
        resolved_types
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
                let is_combinable = self.combine_types(left_ty.clone(), right_ty.clone()).is_ok()
                    || self.combine_types(right_ty.clone(), left_ty.clone()).is_ok();
                if is_combinable {
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
        node_or_token: SyntaxNodeOrToken,
        actual: Ty,
        expected: Ty,
    ) {
        let type_error = TypeError::type_mismatch(node_or_token, expected, actual);
        self.push_type_error(type_error);
    }
}

pub type CombineResult = Result<(), MismatchErrorTypes>;

#[derive(Debug)]
pub struct MismatchErrorTypes {
    _ty1: Ty,
    _ty2: Ty,
}

impl MismatchErrorTypes {
    pub fn new(ty1: Ty, ty2: Ty) -> Self {
        MismatchErrorTypes { _ty1: ty1, _ty2: ty2 }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TypeError {
    TypeMismatch {
        text_range: TextRange,
        expected_ty: Ty,
        actual_ty: Ty,
    },
    UnsupportedOp {
        text_range: TextRange,
        ty: Ty,
        op: String,
    },
    WrongArgumentsToBinExpr {
        text_range: TextRange,
        left_ty: Ty,
        right_ty: Ty,
        op: String,
    },
    InvalidUnpacking {
        text_range: TextRange,
        pat_kind: SyntaxKind,
        assigned_ty: Ty,
    },
    CircularType {
        text_range: TextRange,
        type_name: String,
    },
    WrongArgumentToBorrowExpr {
        text_range: TextRange,
        actual_ty: Ty,
    },
    InvalidDereference {
        text_range: TextRange,
        actual_ty: Ty,
    },
}

impl TypeError {
    pub fn text_range(&self) -> TextRange {
        match self {
            TypeError::TypeMismatch { text_range, .. } => text_range.clone(),
            TypeError::UnsupportedOp { text_range, .. } => text_range.clone(),
            TypeError::WrongArgumentsToBinExpr { text_range, .. } => text_range.clone(),
            TypeError::InvalidUnpacking { text_range, .. } => text_range.clone(),
            TypeError::CircularType { text_range, .. } => text_range.clone(),
            TypeError::WrongArgumentToBorrowExpr { text_range, .. } => text_range.clone(),
            TypeError::InvalidDereference { text_range, .. } => text_range.clone(),
        }
    }

    pub fn type_mismatch(node_or_token: SyntaxNodeOrToken, expected_ty: Ty, actual_ty: Ty) -> Self {
        TypeError::TypeMismatch {
            text_range: node_or_token.text_range(),
            expected_ty,
            actual_ty,
        }
    }

    pub fn unsupported_op(expr: &ast::Expr, ty: Ty, op: ast::BinaryOp) -> Self {
        TypeError::UnsupportedOp {
            text_range: expr.syntax().text_range(),
            ty,
            op: op.to_string(),
        }
    }

    pub fn wrong_arguments_to_bin_expr(
        expr: ast::BinExpr,
        left_ty: Ty,
        right_ty: Ty,
        op: ast::BinaryOp,
    ) -> Self {
        TypeError::WrongArgumentsToBinExpr {
            text_range: expr.syntax().text_range(),
            left_ty,
            right_ty,
            op: op.to_string(),
        }
    }

    pub fn wrong_arguments_to_borrow_expr(inner_expr: ast::Expr, actual_ty: Ty) -> Self {
        TypeError::WrongArgumentToBorrowExpr {
            text_range: inner_expr.syntax().text_range(),
            actual_ty,
        }
    }

    pub fn invalid_dereference(inner_expr: ast::Expr, actual_ty: Ty) -> Self {
        TypeError::InvalidDereference {
            text_range: inner_expr.syntax().text_range(),
            actual_ty,
        }
    }

    pub fn invalid_unpacking(pat: ast::Pat, assigned_ty: Ty) -> Self {
        TypeError::InvalidUnpacking {
            text_range: pat.syntax().text_range(),
            pat_kind: pat.syntax().kind(),
            assigned_ty,
        }
    }

    pub fn circular_type(path: ast::Path, type_name: String) -> Self {
        TypeError::CircularType {
            text_range: path.syntax().text_range(),
            type_name,
        }
    }
}

impl TypeFoldable<TypeError> for TypeError {
    fn deep_fold_with(self, folder: impl TypeFolder) -> TypeError {
        match self {
            TypeError::TypeMismatch {
                text_range,
                expected_ty,
                actual_ty,
            } => TypeError::TypeMismatch {
                text_range,
                expected_ty: expected_ty.fold_with(folder.clone()),
                actual_ty: actual_ty.fold_with(folder),
            },
            TypeError::UnsupportedOp { text_range, ty, op } => TypeError::UnsupportedOp {
                text_range,
                ty: ty.fold_with(folder),
                op,
            },
            TypeError::WrongArgumentsToBinExpr {
                text_range,
                left_ty,
                right_ty,
                op,
            } => TypeError::WrongArgumentsToBinExpr {
                text_range,
                left_ty: left_ty.fold_with(folder.clone()),
                right_ty: right_ty.fold_with(folder),
                op,
            },
            TypeError::InvalidUnpacking {
                text_range,
                pat_kind,
                assigned_ty,
            } => TypeError::InvalidUnpacking {
                text_range,
                pat_kind,
                assigned_ty: assigned_ty.fold_with(folder),
            },
            TypeError::CircularType { .. } => self,
            TypeError::WrongArgumentToBorrowExpr { text_range, actual_ty } => {
                TypeError::WrongArgumentToBorrowExpr {
                    text_range,
                    actual_ty: actual_ty.fold_with(folder),
                }
            }
            TypeError::InvalidDereference { text_range, actual_ty } => TypeError::InvalidDereference {
                text_range,
                actual_ty: actual_ty.fold_with(folder),
            },
        }
    }

    fn deep_visit_with(&self, visitor: impl TypeVisitor) -> bool {
        match self {
            TypeError::TypeMismatch { expected_ty, actual_ty, .. } => {
                visitor.visit_ty(expected_ty) || visitor.visit_ty(actual_ty)
            }
            TypeError::UnsupportedOp { ty, .. } => visitor.visit_ty(ty),
            TypeError::WrongArgumentsToBinExpr { left_ty, right_ty, .. } => {
                visitor.visit_ty(left_ty) || visitor.visit_ty(right_ty)
            }
            TypeError::InvalidUnpacking { assigned_ty, .. } => visitor.visit_ty(assigned_ty),
            TypeError::CircularType { .. } => false,
            TypeError::WrongArgumentToBorrowExpr { actual_ty, .. } => visitor.visit_ty(actual_ty),
            TypeError::InvalidDereference { actual_ty, .. } => visitor.visit_ty(actual_ty),
        }
    }
}
