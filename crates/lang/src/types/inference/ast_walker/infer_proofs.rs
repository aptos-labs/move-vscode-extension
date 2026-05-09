use crate::types::expectation::Expected;
use crate::types::inference::ast_walker::{CallArg, TypeAstWalker};
use crate::types::ty::Ty;
use crate::types::ty::ty_callable::{TyCallable, TyCallableKind};
use syntax::ast;

impl<'a, 'db> TypeAstWalker<'a, 'db> {
    pub(super) fn process_post_stmt(&mut self, post_stmt: &ast::PostStmt) -> Option<()> {
        let inner_stmt = post_stmt.stmt()?;
        self.process_stmt(inner_stmt);
        Some(())
    }

    pub(super) fn process_apply_lemma(&mut self, apply_lemma: &ast::ApplyLemma) -> Option<()> {
        let lemma_path_expr = apply_lemma.path_expr()?;
        let path_ty = self.infer_path_expr(&lemma_path_expr, Expected::NoValue);
        let callable_ty = match path_ty {
            Some(Ty::Callable(ty_callable)) => ty_callable,
            _ => TyCallable::fake(
                apply_lemma.to_any_call_expr().n_provided_args(),
                TyCallableKind::fake(),
            ),
        };
        let expected_arg_tys = self.infer_expected_call_arg_tys(&callable_ty, Expected::NoValue);
        let args = apply_lemma
            .to_any_call_expr()
            .arg_exprs()
            .into_iter()
            .map(|expr| CallArg::Arg { expr })
            .collect();
        self.coerce_call_arg_types(args, callable_ty.param_types.clone(), expected_arg_tys);

        self.ctx
            .call_expr_types
            .insert(apply_lemma.clone().into(), callable_ty.clone().into());

        // resolve after applying all parameters
        // let ret_ty = self.ctx.resolve_ty_vars_if_possible(callable_ty.ret_type_ty());
        // Some(ret_ty)
        Some(())
    }
}
