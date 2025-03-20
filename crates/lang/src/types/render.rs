use crate::loc::SyntaxLoc;
use crate::types::ty::adt::TyAdt;
use crate::types::ty::ty_var::{TyInfer, TyVar, TyVarKind};
use crate::types::ty::type_param::TyTypeParameter;
use crate::types::ty::Ty;
use crate::AsName;
use base_db::SourceRootDatabase;
use stdx::itertools::Itertools;
use syntax::ast;
use syntax::ast::HasName;

pub struct TypeRenderer<'db> {
    db: &'db dyn SourceRootDatabase,
}

impl<'db> TypeRenderer<'db> {
    pub fn new(db: &'db dyn SourceRootDatabase) -> Self {
        TypeRenderer { db }
    }

    pub fn render(&self, ty: &Ty) -> String {
        match ty {
            Ty::Vector(ty) => {
                format!("vector<{}>", self.render(ty))
            }
            Ty::Adt(ty_adt) => self.render_ty_adt(ty_adt),
            Ty::Reference(ty_ref) => {
                let prefix = if ty_ref.is_mut() { "&mut " } else { "&" };
                let inner = self.render(ty_ref.referenced());
                format!("{}{}", prefix, inner)
            }
            Ty::Tuple(ty_tuple) => {
                let rendered_tys = ty_tuple.types.iter().map(|it| self.render(it)).join(", ");
                format!("({})", rendered_tys)
            }

            Ty::TypeParam(ty_tp) => self.render_ty_tp(ty_tp),
            Ty::Infer(ty_infer) => match ty_infer {
                TyInfer::Var(ty_var) => self.render_ty_var(ty_var),
                TyInfer::IntVar(_) => "?integer".to_string(),
            },

            Ty::Bool => "bool".to_string(),
            Ty::Signer => "signer".to_string(),
            Ty::Address => "address".to_string(),
            Ty::Integer(kind) => kind.to_string(),
            Ty::Num => "num".to_string(),

            Ty::Unit => "()".to_string(),
            Ty::Unknown => unknown(),
            Ty::Never => never(),
        }
    }

    fn render_ty_tp(&self, type_param: &TyTypeParameter) -> String {
        self.origin_loc_name(type_param.origin_loc)
    }

    fn render_ty_var(&self, ty_var: &TyVar) -> String {
        let kind = match ty_var.kind {
            TyVarKind::Anonymous(index) => index.to_string(),
            TyVarKind::WithOrigin { origin_loc } => self.origin_loc_name(origin_loc),
        };
        format!("?_{}", kind)
    }

    fn render_ty_adt(&self, _ty_adt: &TyAdt) -> String {
        "_".to_string()
    }

    fn origin_loc_name(&self, origin_loc: SyntaxLoc) -> String {
        origin_loc
            .cast::<ast::TypeParam>(self.db)
            .and_then(|tp| tp.value.name())
            .map(|tp_name| tp_name.as_name().to_string())
            .unwrap_or(anonymous())
    }
}

fn unknown() -> String {
    "<unknown>".to_string()
}

fn never() -> String {
    "<never>".to_string()
}

fn anonymous() -> String {
    "<anonymous>".to_string()
}
