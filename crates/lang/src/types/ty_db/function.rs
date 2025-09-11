use crate::loc::{SyntaxLocFileExt, SyntaxLocInput};
use crate::types::ty::Ty;
use crate::types::ty::ty_callable::{TyCallable, TyCallableKind};
use crate::types::ty_db::lower_type;
use base_db::SourceDatabase;
use syntax::ast;
use syntax::files::{InFile, InFileExt};

pub fn lower_function(db: &dyn SourceDatabase, fun: InFile<ast::AnyFun>, msl: bool) -> TyCallable {
    let fun_loc = SyntaxLocInput::new(db, fun.loc());
    lower_function_tracked(db, fun_loc, msl)
}

#[salsa_macros::tracked]
fn lower_function_tracked<'db>(
    db: &'db dyn SourceDatabase,
    fun_loc: SyntaxLocInput<'db>,
    msl: bool,
) -> TyCallable {
    let any_fun_loc = fun_loc.syntax_loc(db);
    let any_fun = any_fun_loc
        .to_ast::<ast::AnyFun>(db)
        .expect("might be a stale cache issue");

    let item_subst = any_fun.ty_type_params_subst();
    let (file_id, any_fun) = any_fun.unpack();
    let param_types = any_fun
        .params()
        .into_iter()
        .map(|it| {
            it.type_()
                .map(|t| lower_type(db, t.in_file(file_id), msl))
                .unwrap_or(Ty::Unknown)
        })
        .collect();
    let ret_type = any_fun.ret_type().map(|t| t.in_file(file_id));
    let ret_type_ty = match ret_type {
        Some(ret_type) => ret_type
            .and_then(|it| it.type_())
            .map(|t| lower_type(db, t, msl))
            .unwrap_or(Ty::Unknown),
        None => Ty::Unit,
    };
    TyCallable::new(
        param_types,
        ret_type_ty,
        TyCallableKind::named(item_subst, Some(any_fun_loc)),
    )
}
