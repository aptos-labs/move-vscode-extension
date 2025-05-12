pub mod adt;
pub(crate) mod integer;
pub(crate) mod range_like;
pub mod reference;
pub(crate) mod schema;
pub(crate) mod tuple;
pub mod ty_callable;
pub(crate) mod ty_var;
pub(crate) mod type_param;

use crate::loc::SyntaxLoc;
use crate::nameres::address::Address;
use crate::nameres::name_resolution::get_modules_as_entries;
use crate::nameres::scope::{ScopeEntryListExt, VecExt};
use crate::types::fold::{TypeFoldable, TypeFolder, TypeVisitor};
use crate::types::inference::InferenceCtx;
use crate::types::render::TypeRenderer;
use crate::types::ty::adt::TyAdt;
use crate::types::ty::integer::IntegerKind;
use crate::types::ty::range_like::TySequence;
use crate::types::ty::reference::{Mutability, TyReference};
use crate::types::ty::schema::TySchema;
use crate::types::ty::tuple::TyTuple;
use crate::types::ty::ty_callable::TyCallable;
use crate::types::ty::ty_var::{TyInfer, TyVar};
use crate::types::ty::type_param::TyTypeParameter;
use base_db::SourceDatabase;
use base_db::package_root::PackageId;
use syntax::ast;
use syntax::files::InFile;
use vfs::FileId;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Ty {
    Unknown,
    Never,

    Unit,
    Bool,
    Signer,
    Address,
    Integer(IntegerKind),
    Num,

    Seq(TySequence),

    Infer(TyInfer),
    TypeParam(TyTypeParameter),

    Reference(TyReference),
    Adt(TyAdt),
    Callable(TyCallable),
    Tuple(TyTuple),

    Schema(TySchema),
}

impl Ty {
    pub fn new_ty_var(ctx: &mut InferenceCtx) -> Ty {
        Ty::Infer(TyInfer::Var(TyVar::new_anonymous(ctx.inc_ty_counter())))
    }

    pub fn new_ty_var_with_origin(tp_origin_loc: SyntaxLoc) -> Ty {
        Ty::Infer(TyInfer::Var(TyVar::new_with_origin(tp_origin_loc)))
    }

    pub fn new_vector(item_ty: Ty) -> Ty {
        Ty::Seq(TySequence::Vector(Box::new(item_ty)))
    }

    pub fn new_tuple(tys: Vec<Ty>) -> Ty {
        Ty::Tuple(TyTuple::new(tys))
    }

    pub fn new_ty_adt(item: InFile<ast::StructOrEnum>) -> Ty {
        Ty::Adt(TyAdt::new(item))
    }

    pub fn new_reference(inner_ty: Ty, mutability: Mutability) -> Ty {
        Ty::Reference(TyReference::new(inner_ty, mutability))
    }

    pub fn adt_item_module(
        &self,
        db: &dyn SourceDatabase,
        current_package_id: PackageId,
    ) -> Option<InFile<ast::Module>> {
        let ty = self.unwrap_all_refs();
        match ty {
            Ty::Adt(ty_adt) => {
                let item = ty_adt.adt_item_loc.to_ast::<ast::StructOrEnum>(db)?;
                Some(item.map(|it| it.module()))
            }
            Ty::Seq(TySequence::Vector(_)) => {
                let module = get_modules_as_entries(db, current_package_id, Address::named("std"))
                    .filter_by_name("vector".to_string())
                    .single_or_none()?;
                module.cast_into::<ast::Module>(db)
            }
            _ => None,
        }
    }

    pub fn into_ty_callable(self) -> Option<TyCallable> {
        match self {
            Ty::Callable(ty_callable) => Some(ty_callable),
            _ => None,
        }
    }

    pub fn into_ty_ref(self) -> Option<TyReference> {
        match self {
            Ty::Reference(ty_ref) => Some(ty_ref),
            _ => None,
        }
    }

    pub fn into_ty_adt(self) -> Option<TyAdt> {
        match self {
            Ty::Adt(ty_adt) => Some(ty_adt),
            _ => None,
        }
    }

    pub fn into_ty_schema(self) -> Option<TySchema> {
        match self {
            Ty::Schema(ty_schema) => Some(ty_schema),
            _ => None,
        }
    }

    pub fn into_ty_tuple(self) -> Option<TyTuple> {
        match self {
            Ty::Tuple(ty_tuple) => Some(ty_tuple),
            _ => None,
        }
    }

    pub fn into_ty_seq(self) -> Option<TySequence> {
        match self {
            Ty::Seq(ty_seq) => Some(ty_seq),
            _ => None,
        }
    }

    pub fn refine_for_specs(self, msl: bool) -> Ty {
        let mut ty = self;
        if !msl {
            return ty;
        }
        if matches!(ty, Ty::Reference(_)) {
            ty = ty.unwrap_all_refs();
        }
        if matches!(ty, Ty::Integer(_) | Ty::Infer(TyInfer::IntVar(_))) {
            ty = Ty::Num;
        }
        ty
    }

    pub fn unwrap_all_refs(&self) -> Ty {
        match self {
            Ty::Reference(ty_ref) => ty_ref.referenced().unwrap_all_refs(),
            _ => self.to_owned(),
        }
    }

    pub fn render(&self, db: &dyn SourceDatabase, context_file_id: Option<FileId>) -> String {
        TypeRenderer::new(db, context_file_id).render(self)
    }
}

impl TypeFoldable<Ty> for Ty {
    fn fold_with(self, folder: impl TypeFolder) -> Ty {
        folder.fold_ty(self)
    }

    fn visit_with(&self, visitor: impl TypeVisitor) -> bool {
        visitor.visit_ty(self)
    }

    fn deep_fold_with(self, folder: impl TypeFolder) -> Ty {
        match self {
            Ty::Adt(ty_adt) => Ty::Adt(ty_adt.deep_fold_with(folder)),
            Ty::Seq(ty_seq) => Ty::Seq(ty_seq.deep_fold_with(folder)),
            Ty::Reference(ty_ref) => Ty::Reference(TyReference::new(
                folder.fold_ty(ty_ref.referenced()),
                ty_ref.mutability,
            )),
            Ty::Callable(ty_callable) => Ty::Callable(ty_callable.deep_fold_with(folder)),
            _ => self,
        }
    }

    fn deep_visit_with(&self, visitor: impl TypeVisitor) -> bool {
        match self {
            Ty::Adt(ty_adt) => ty_adt.deep_visit_with(visitor),
            Ty::Seq(ty_seq) => ty_seq.deep_visit_with(visitor),
            Ty::Reference(ty_ref) => visitor.visit_ty(&ty_ref.referenced()),
            Ty::Callable(ty_callable) => ty_callable.deep_visit_with(visitor),
            _ => false,
        }
    }
}
