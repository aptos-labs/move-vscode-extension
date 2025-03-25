mod type_args;

use crate::db::HirDatabase;
use crate::files::{InFileExt, InFileInto};
use crate::types::substitution::ApplySubstitution;
use crate::types::ty::adt::TyAdt;
use crate::types::ty::integer::IntegerKind;
use crate::types::ty::reference::{Mutability, TyReference};
use crate::types::ty::tuple::TyTuple;
use crate::types::ty::ty_callable::TyCallable;
use crate::types::ty::type_param::TyTypeParameter;
use crate::types::ty::Ty;
use crate::InFile;
use parser::SyntaxKind::{ENUM, STRUCT, TYPE_PARAM};
use syntax::{ast, AstNode, SyntaxNode};
use vfs::FileId;

pub struct TyLowering<'a> {
    db: &'a dyn HirDatabase,
    file_id: FileId,
}

impl<'a> TyLowering<'a> {
    pub fn new(db: &'a dyn HirDatabase, file_id: FileId) -> Self {
        TyLowering { db, file_id }
    }

    pub fn lower_type(&self, type_: ast::Type) -> Ty {
        match type_ {
            ast::Type::PathType(path_type) => {
                let path = path_type.path();
                let named_item = path.clone().in_file(self.file_id).resolve_no_inf(self.db);
                match named_item {
                    None => {
                        // can be primitive type
                        self.lower_primitive_type(path).unwrap_or(Ty::Unknown)
                    }
                    Some(named_item_entry) => {
                        let named_item = named_item_entry
                            .node_loc
                            .cast_into::<ast::AnyNamedElement>(self.db.upcast())
                            .unwrap();
                        self.lower_path(path.into(), named_item.map(|it| it.syntax().to_owned()))
                    }
                }
            }
            ast::Type::RefType(ref_type) => {
                let is_mut = ref_type.is_mut();
                let inner_ty = ref_type
                    .type_()
                    .map(|inner_type| self.lower_type(inner_type))
                    .unwrap_or(Ty::Unknown);
                Ty::Reference(TyReference::new(inner_ty, Mutability::new(is_mut)))
            }
            ast::Type::TupleType(tuple_type) => {
                let inner_tys = tuple_type
                    .types()
                    .map(|it| self.lower_type(it))
                    .collect::<Vec<_>>();
                Ty::Tuple(TyTuple::new(inner_tys))
            }
            ast::Type::UnitType(_) => Ty::Unit,
            ast::Type::ParenType(paren_type) => self.lower_type(paren_type.type_()),
        }
    }

    pub fn lower_path(&self, method_or_path: ast::MethodOrPath, named_item: InFile<SyntaxNode>) -> Ty {
        use syntax::SyntaxKind::*;

        let path_ty = match named_item.syntax_kind() {
            TYPE_PARAM => {
                let type_param = named_item.clone().syntax_cast::<ast::TypeParam>().unwrap();
                Ty::TypeParam(TyTypeParameter::new(type_param))
            }
            STRUCT | ENUM => {
                let item = named_item.clone().syntax_cast::<ast::StructOrEnum>().unwrap();
                Ty::Adt(TyAdt::new(item))
            }
            FUN => {
                let fun = named_item.clone().syntax_cast::<ast::Fun>().unwrap();
                let ty_callable = self.lower_function(fun.value);
                Ty::Callable(ty_callable)
            }
            VARIANT => {
                let variant = named_item.clone().syntax_cast::<ast::Variant>().unwrap();
                let enum_ = variant.map(|it| it.enum_());
                let Some(enum_path) = method_or_path
                    .clone()
                    .path()
                    .expect("MethodCallExpr cannot be resolved to Variant")
                    .qualifier()
                else {
                    return Ty::Unknown;
                };
                self.lower_path(
                    ast::MethodOrPath::Path(enum_path),
                    enum_.map(|it| it.syntax().to_owned()),
                )
            }
            _ => Ty::Unknown,
        };

        // adds associations of ?Element -> (type of ?Element from explicitly set types)
        // Option<u8>: ?Element -> u8
        // Option: ?Element -> ?Element
        if let Some(generic_item) = named_item.syntax_cast::<ast::AnyGenericItem>() {
            let type_args_subst = self.type_args_substitution(method_or_path, generic_item);
            return path_ty.substitute(type_args_subst);
        }

        path_ty
    }

    pub fn lower_function(&self, fun: ast::Fun) -> TyCallable {
        let param_types = fun
            .params()
            .into_iter()
            .map(|it| it.type_().map(|t| self.lower_type(t)).unwrap_or(Ty::Unknown))
            .collect();
        let ret_type = self.lower_ret_type(fun.ret_type());
        TyCallable::new(param_types, ret_type)
    }

    fn lower_ret_type(&self, ret_type: Option<ast::RetType>) -> Ty {
        let Some(ret_type) = ret_type else {
            return Ty::Unit;
        };
        ret_type
            .type_()
            .map(|t| self.lower_type(t))
            .unwrap_or(Ty::Unknown)
    }

    fn lower_primitive_type(&self, path: ast::Path) -> Option<Ty> {
        let path_name = path.reference_name()?;
        let ty = match path_name.as_str() {
            "u8" => Ty::Integer(IntegerKind::U8),
            "u16" => Ty::Integer(IntegerKind::U16),
            "u32" => Ty::Integer(IntegerKind::U32),
            "u64" => Ty::Integer(IntegerKind::U64),
            "u128" => Ty::Integer(IntegerKind::U128),
            "u256" => Ty::Integer(IntegerKind::U256),
            "bool" => Ty::Bool,
            "signer" => Ty::Signer,
            "address" => Ty::Address,
            "vector" => {
                let arg_ty = path
                    .type_args()
                    .first()
                    .map(|it| self.lower_type(it.type_()))
                    .unwrap_or(Ty::Unknown);
                Ty::Vector(Box::new(arg_ty))
            }
            _ => {
                return None;
            }
        };
        Some(ty)
    }
}
