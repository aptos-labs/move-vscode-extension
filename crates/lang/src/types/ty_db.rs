use crate::loc::{SyntaxLoc, SyntaxLocFileExt, SyntaxLocInput, SyntaxLocNodeExt};
use crate::nameres;
use crate::types::has_type_params_ext::GenericItemExt;
use crate::types::lowering::TyLowering;
use crate::types::ty::Ty;
use crate::types::ty::integer::IntegerKind;
use crate::types::ty::range_like::TySequence;
use crate::types::ty::reference::Mutability;
use crate::types::ty::tuple::TyTuple;
use crate::types::ty::ty_callable::{TyCallable, TyCallableKind};
use crate::types::ty_db;
use base_db::SourceDatabase;
use syntax::ast;
use syntax::ast::idents::INTEGER_IDENTS;
use syntax::files::{InFile, InFileExt};

pub(crate) fn lower_type(db: &dyn SourceDatabase, type_: InFile<ast::Type>, msl: bool) -> Ty {
    let _p = tracing::debug_span!("ty_db::lower_type").entered();
    lower_type_inner(db, type_, msl).unwrap_or(Ty::Unknown)
}

pub(crate) fn lower_type_inner(
    db: &dyn SourceDatabase,
    type_: InFile<ast::Type>,
    msl: bool,
) -> Option<Ty> {
    let type_loc = SyntaxLocInput::new(db, type_.loc());
    lower_type_tracked(db, type_loc, msl)
}

#[salsa_macros::tracked]
fn lower_type_tracked<'db>(
    db: &'db dyn SourceDatabase,
    type_loc: SyntaxLocInput<'db>,
    msl: bool,
) -> Option<Ty> {
    let _p = tracing::debug_span!("ty_db::lower_type_tracked").entered();

    let type_ = type_loc.to_ast::<ast::Type>(db)?;
    let lowering = TyLowering::new(db, msl);

    lowering.lower_type_inner(type_)
}

pub fn lower_function(db: &dyn SourceDatabase, fun: InFile<ast::AnyFun>, msl: bool) -> TyCallable {
    let _p = tracing::debug_span!("ty_db::lower_function").entered();

    let fun_loc = SyntaxLocInput::new(db, fun.loc());
    lower_function_tracked(db, fun_loc, msl)
}

#[salsa_macros::tracked]
fn lower_function_tracked<'db>(
    db: &'db dyn SourceDatabase,
    fun_loc: SyntaxLocInput<'db>,
    msl: bool,
) -> TyCallable {
    let _p = tracing::debug_span!("ty_db::lower_function_tracked").entered();

    let any_fun = fun_loc
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
        TyCallableKind::named(item_subst, Some(any_fun.loc(file_id))),
    )
}

pub fn lower_primitive_type(db: &dyn SourceDatabase, path: InFile<ast::Path>, msl: bool) -> Option<Ty> {
    let _p = tracing::debug_span!("ty_db::lower_primitive_type").entered();

    let path_loc = SyntaxLocInput::new(db, path.loc());
    lower_primitive_type_tracked(db, path_loc, msl)
}

#[salsa_macros::tracked]
fn lower_primitive_type_tracked<'db>(
    db: &'db dyn SourceDatabase,
    path_loc: SyntaxLocInput<'db>,
    msl: bool,
) -> Option<Ty> {
    let _p = tracing::debug_span!("ty_db::lower_primitive_type_tracked").entered();

    let (file_id, path) = path_loc.to_ast::<ast::Path>(db)?.unpack();

    let path_name = path.reference_name()?;
    if msl && INTEGER_IDENTS.contains(&path_name.as_str()) {
        return Some(Ty::Num);
    }

    let ty = match path_name.as_str() {
        "u8" => Ty::Integer(IntegerKind::U8),
        "u16" => Ty::Integer(IntegerKind::U16),
        "u32" => Ty::Integer(IntegerKind::U32),
        "u64" => Ty::Integer(IntegerKind::U64),
        "u128" => Ty::Integer(IntegerKind::U128),
        "u256" => Ty::Integer(IntegerKind::U256),
        "num" => Ty::Num,
        "bv" => Ty::Bv,
        "bool" => Ty::Bool,
        "signer" => Ty::Signer,
        "address" => Ty::Address,
        "vector" => {
            let first_arg_type = path.type_args().first().and_then(|it| it.type_());
            let first_arg_ty = first_arg_type
                .map(|it| lower_type(db, it.in_file(file_id), msl))
                .unwrap_or(Ty::Unknown);
            Ty::new_vector(first_arg_ty)
        }
        "range" => {
            let first_arg_type = path.type_args().first().and_then(|it| it.type_());
            let first_arg_ty = first_arg_type
                .map(|it| lower_type(db, it.in_file(file_id), msl))
                .unwrap_or(Ty::Unknown);
            Ty::Seq(TySequence::Range(Box::new(first_arg_ty)))
        }
        _ => {
            return None;
        }
    };
    Some(ty)
}
