use crate::types::inference::TypeError;
use crate::types::substitution::{ApplySubstitution, Substitution};
use crate::types::ty::Ty;
use crate::types::ty::adt::TyAdt;
use crate::types::ty::schema::TySchema;
use crate::types::ty::type_param::TyTypeParameter;
use crate::types::ty_db;
use base_db::SourceDatabase;
use std::collections::HashMap;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, SyntaxKind::*, ast};

pub fn lower_path(
    db: &dyn SourceDatabase,
    method_or_path: InFile<ast::MethodOrPath>,
    named_item: InFile<impl Into<ast::NamedElement>>,
    msl: bool,
) -> (Ty, Vec<TypeError>) {
    let named_item = named_item.map(|it| it.into());
    let path_ty = match named_item.kind() {
        TYPE_PARAM => {
            let type_param = named_item.clone().cast_into::<ast::TypeParam>().unwrap();
            Ty::TypeParam(TyTypeParameter::new(type_param))
        }
        STRUCT | ENUM => {
            let item = named_item.clone().cast_into::<ast::StructOrEnum>().unwrap();
            Ty::Adt(TyAdt::new(item))
        }
        SCHEMA => {
            let item = named_item.clone().cast_into::<ast::Schema>().unwrap();
            Ty::Schema(TySchema::new(item))
        }
        FUN | SPEC_FUN | SPEC_INLINE_FUN => {
            let fun = named_item.clone().cast_into::<ast::AnyFun>().unwrap();
            let ty_callable = ty_db::lower_function(db, fun, msl);
            Ty::Callable(ty_callable)
        }
        VARIANT => {
            let variant = named_item.clone().cast_into::<ast::Variant>().unwrap();
            let enum_ = variant.map(|it| it.enum_());
            let (file_id, method_or_path) = method_or_path.clone().unpack();
            let Some(enum_path) = method_or_path
                .path()
                .expect("MethodCallExpr cannot be resolved to Variant")
                .qualifier()
            else {
                return (Ty::Unknown, vec![]);
            };
            let (variant_path_ty, _) = lower_path(db, enum_path.in_file(file_id).map_into(), enum_, msl);
            // todo: ability checks for enum variants
            variant_path_ty
        }
        _ => Ty::Unknown,
    };

    // adds associations of ?Element -> (type of ?Element from explicitly set types)
    // Option<u8>: ?Element -> u8
    // Option: ?Element -> ?Element
    if let Some(generic_item) = named_item.cast_into::<ast::GenericElement>() {
        let (type_args_subst, type_errors) =
            type_args_substitution(db, msl, method_or_path.as_ref(), generic_item.as_ref());
        return (path_ty.substitute(&type_args_subst), type_errors);
    }

    (path_ty, vec![])
}

fn type_args_substitution(
    db: &dyn SourceDatabase,
    msl: bool,
    method_or_path: InFile<&ast::MethodOrPath>,
    generic_item: InFile<&ast::GenericElement>,
) -> (Substitution, Vec<TypeError>) {
    let (method_or_path_file_id, method_or_path) = method_or_path.unpack();

    let psi_subst = psi_type_args_subst(method_or_path, generic_item.value);

    let mut subst_mapping = HashMap::new();
    let mut missing_ability_errors = vec![];
    for (type_param, psi_type_arg) in psi_subst {
        let type_param = InFile::new(generic_item.file_id, type_param);
        let type_param_ty = TyTypeParameter::new(type_param.clone());
        let ty = match psi_type_arg {
            PsiTypeArg::Present(type_) => {
                let type_arg_ty =
                    ty_db::lower_type(db, type_.clone().in_file(method_or_path_file_id), msl);
                if !msl
                    && let (Some(required_abilities), Some(type_arg_abilities)) =
                        (type_param_ty.abilities(db), type_arg_ty.abilities(db))
                {
                    let mut missing_abilities = vec![];
                    for required_ability in required_abilities.iter() {
                        if !type_arg_abilities.contains(required_ability) {
                            missing_abilities.push(required_ability.clone());
                        }
                    }
                    if !missing_abilities.is_empty() {
                        missing_ability_errors.push(TypeError::missing_abilities(
                            type_.syntax().clone().into(),
                            type_arg_ty.clone(),
                            missing_abilities,
                        ));
                    }
                }
                type_arg_ty
            }
            PsiTypeArg::OptionalAbsent => Ty::TypeParam(type_param_ty.clone()),
            PsiTypeArg::RequiredAbsent => Ty::Unknown,
        };
        subst_mapping.insert(type_param_ty, ty);
    }

    (Substitution::new(subst_mapping), missing_ability_errors)
}

#[derive(Debug, Clone)]
pub enum PsiTypeArg {
    Present(ast::Type),
    RequiredAbsent,
    OptionalAbsent,
}

fn psi_type_args_subst(
    method_or_path: &ast::MethodOrPath,
    generic_item: &ast::GenericElement,
) -> HashMap<ast::TypeParam, PsiTypeArg> {
    let is_args_optional = match method_or_path {
        ast::MethodOrPath::Path(path) => {
            let path_context = path.root_path().syntax().parent().unwrap();
            ast::Expr::can_cast(path_context.kind()) || ast::Pat::can_cast(path_context.kind())
        }
        ast::MethodOrPath::MethodCallExpr(_) => true,
    };

    // Generic arguments are optional in expression context, e.g.
    // `let a = Foo::<u8>::bar::<u16>();` can be written as `let a = Foo::bar();`
    // if it is possible to infer `u8` and `u16` during type inference

    let type_params = generic_item.type_params();
    let type_args_list = method_or_path.type_arg_list();
    if type_args_list.is_none() {
        let type_arg = if is_args_optional {
            PsiTypeArg::OptionalAbsent
        } else {
            PsiTypeArg::RequiredAbsent
        };
        return type_params.into_iter().map(|it| (it, type_arg.clone())).collect();
    }

    let mut type_args = type_args_list
        .unwrap()
        .type_arguments()
        .collect::<Vec<_>>()
        .into_iter();
    let mut subst = HashMap::new();
    for type_param in type_params {
        let type_arg = type_args.next().and_then(|it| it.type_());
        let psi_type_arg = match type_arg {
            Some(type_arg) => PsiTypeArg::Present(type_arg),
            None => PsiTypeArg::RequiredAbsent,
        };
        subst.insert(type_param, psi_type_arg);
    }
    subst
}
