mod function;
mod path;

use crate::loc::{SyntaxLocFileExt, SyntaxLocInput, SyntaxLocNodeExt};
use crate::nameres;
use crate::types::inference::InferenceCtx;
use crate::types::ty::Ty;
use crate::types::ty::integer::IntegerKind;
use crate::types::ty::range_like::TySequence;
use crate::types::ty::reference::Mutability;
use crate::types::ty::tuple::TyTuple;
use crate::types::ty::ty_callable::{TyCallable, TyCallableKind};
use base_db::SourceDatabase;
use syntax::ast;
use syntax::ast::idents::INTEGER_IDENTS;
use syntax::files::{InFile, InFileExt};

pub(crate) fn lower_type_for_ctx(ctx: &InferenceCtx, type_: InFile<ast::Type>) -> Ty {
    try_lower_type(ctx.db, type_, ctx.msl).unwrap_or(Ty::Unknown)
}

pub(crate) fn lower_type(db: &dyn SourceDatabase, type_: InFile<ast::Type>, msl: bool) -> Ty {
    try_lower_type(db, type_, msl).unwrap_or(Ty::Unknown)
}

pub(crate) fn try_lower_type(
    db: &dyn SourceDatabase,
    type_: InFile<ast::Type>,
    msl: bool,
) -> Option<Ty> {
    let _p = tracing::debug_span!("ty_db::try_lower_type").entered();
    let type_loc = SyntaxLocInput::new(db, type_.loc());
    try_lower_type_tracked(db, type_loc, msl)
}

#[salsa_macros::tracked]
fn try_lower_type_tracked<'db>(
    db: &'db dyn SourceDatabase,
    type_loc: SyntaxLocInput<'db>,
    msl: bool,
) -> Option<Ty> {
    let (file_id, type_) = type_loc.to_ast::<ast::Type>(db)?.unpack();
    match type_ {
        ast::Type::PathType(path_type) => {
            let path = path_type.path().in_file(file_id);
            let named_item = nameres::resolve_no_inf(db, path.clone());
            match named_item {
                None => {
                    // can still be primitive type
                    lower_primitive_type(db, path, msl)
                }
                Some(named_item_entry) => {
                    let named_element = named_item_entry.node_loc.to_ast::<ast::NamedElement>(db)?;
                    let (path_type_ty, _) = lower_path(db, path.map_into(), named_element, msl);
                    // todo: ability checks in types
                    Some(path_type_ty)
                }
            }
        }
        ast::Type::RefType(ref_type) => {
            let is_mut = ref_type.is_mut();
            let inner_ty = ref_type
                .type_()
                .map(|inner_type| lower_type(db, inner_type.in_file(file_id), msl))
                .unwrap_or(Ty::Unknown);
            Some(Ty::new_reference(inner_ty, Mutability::new(is_mut)))
        }
        ast::Type::TupleType(tuple_type) => {
            let inner_tys = tuple_type
                .types()
                .map(|inner_type| lower_type(db, inner_type.in_file(file_id), msl))
                .collect::<Vec<_>>();
            Some(Ty::Tuple(TyTuple::new(inner_tys)))
        }
        ast::Type::UnitType(_) => Some(Ty::Unit),
        ast::Type::ParenType(paren_type) => {
            let paren_ty = paren_type.type_()?.in_file(file_id);
            try_lower_type(db, paren_ty, msl)
        }
        ast::Type::LambdaType(lambda_type) => {
            let param_tys = lambda_type
                .param_types()
                .into_iter()
                .map(|it| lower_type(db, it.in_file(file_id), msl))
                .collect();
            let ret_ty = lambda_type
                .return_type()
                .map(|it| lower_type(db, it.in_file(file_id), msl))
                .unwrap_or(Ty::Unit);
            Some(Ty::Callable(TyCallable::new(
                param_tys,
                ret_ty,
                TyCallableKind::Lambda(Some(lambda_type.loc(file_id))),
            )))
        }
    }
}

pub fn lower_type_owner_for_ctx(
    ctx: &InferenceCtx,
    type_owner: InFile<impl Into<ast::TypeOwner>>,
) -> Option<Ty> {
    let type_owner = type_owner.map(|it| it.into());
    type_owner
        .and_then(|it| it.type_())
        .map(|type_| lower_type(ctx.db, type_, ctx.msl))
}

pub fn lower_type_owner(
    db: &dyn SourceDatabase,
    type_owner: InFile<impl Into<ast::TypeOwner>>,
    msl: bool,
) -> Option<Ty> {
    let type_owner = type_owner.map(|it| it.into());
    type_owner
        .and_then(|it| it.type_())
        .map(|type_| lower_type(db, type_, msl))
}

pub use function::lower_function;
pub use path::lower_path;

pub fn lower_primitive_type(db: &dyn SourceDatabase, path: InFile<ast::Path>, msl: bool) -> Option<Ty> {
    let path_loc = SyntaxLocInput::new(db, path.loc());
    lower_primitive_type_tracked(db, path_loc, msl)
}

#[salsa_macros::tracked]
fn lower_primitive_type_tracked<'db>(
    db: &'db dyn SourceDatabase,
    path_loc: SyntaxLocInput<'db>,
    msl: bool,
) -> Option<Ty> {
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
