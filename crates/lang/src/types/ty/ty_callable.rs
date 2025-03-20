use crate::types::fold::{TypeFoldable, TypeFolder, TypeVisitor};
use crate::types::inference::InferenceCtx;
use crate::types::ty::reference::autoborrow;
use crate::types::ty::Ty;
use std::iter;
use syntax::ast;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TyCallable {
    pub param_types: Vec<Ty>,
    pub ret_type: Box<Ty>,
}

impl TyCallable {
    pub fn new(param_types: Vec<Ty>, ret_type: Ty) -> Self {
        TyCallable {
            param_types,
            ret_type: Box::new(ret_type),
        }
    }

    pub fn fake(n_params: usize) -> Self {
        TyCallable {
            param_types: iter::repeat_n(Ty::Unknown, n_params).collect(),
            ret_type: Box::new(Ty::Unknown),
        }
    }
}

impl TypeFoldable<TyCallable> for TyCallable {
    fn deep_fold_with(self, folder: impl TypeFolder) -> TyCallable {
        let TyCallable {
            param_types,
            ret_type,
        } = self;
        TyCallable::new(folder.fold_tys(param_types), folder.fold_ty(*ret_type))
    }

    fn deep_visit_with(&self, visitor: impl TypeVisitor) -> bool {
        visitor.visit_tys(&self.param_types) || visitor.visit_ty(&self.ret_type)
    }
}
