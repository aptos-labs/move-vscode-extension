use crate::loc::SyntaxLoc;
use crate::nameres::fq_named_element::ItemFQNameOwner;
use crate::types::ty::Ty;
use crate::types::ty::adt::TyAdt;
use crate::types::ty::range_like::TySequence;
use crate::types::ty::schema::TySchema;
use crate::types::ty::ty_callable::{CallKind, TyCallable};
use crate::types::ty::ty_var::{TyInfer, TyVar, TyVarKind};
use crate::types::ty::type_param::TyTypeParameter;
use base_db::SourceDatabase;
use stdx::itertools::Itertools;
use syntax::ast;
use syntax::files::InFile;
use vfs::FileId;

pub struct TypeRenderer<'db> {
    db: &'db dyn SourceDatabase,
    context_file_id: Option<FileId>,
}

impl<'db> TypeRenderer<'db> {
    pub fn new(db: &'db dyn SourceDatabase, context: Option<FileId>) -> Self {
        TypeRenderer { db, context_file_id: context }
    }

    pub fn render(&self, ty: &Ty) -> String {
        match ty {
            Ty::Seq(ty_seq) => {
                let type_name = match ty_seq {
                    TySequence::Vector(_) => "vector",
                    TySequence::Range(_) => "range",
                };
                format!("{}<{}>", type_name, self.render(&ty_seq.item()))
            }
            Ty::Adt(ty_adt) => self.render_ty_adt(ty_adt),
            Ty::Schema(ty_schema) => self.render_ty_schema(ty_schema),
            Ty::Callable(ty_callable) => self.render_ty_callable(ty_callable),
            Ty::Reference(ty_ref) => {
                let prefix = if ty_ref.is_mut() { "&mut " } else { "&" };
                let inner = self.render(&ty_ref.referenced());
                format!("{}{}", prefix, inner)
            }
            Ty::Tuple(ty_tuple) => {
                let rendered_tys = self.render_list(&ty_tuple.types, ", ");
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
            Ty::Bv => "bv".to_string(),

            Ty::Unit => "()".to_string(),
            Ty::Unknown => unknown(),
            Ty::Never => never(),
        }
    }

    fn render_list(&self, tys: &Vec<Ty>, sep: &str) -> String {
        tys.iter().map(|it| self.render(it)).join(sep)
    }

    fn render_ty_tp(&self, type_param: &TyTypeParameter) -> String {
        self.origin_loc_name(&type_param.origin_loc)
    }

    fn render_ty_var(&self, ty_var: &TyVar) -> String {
        match &ty_var.kind {
            TyVarKind::Anonymous(index) => format!("?_{index}"),
            TyVarKind::WithOrigin { origin_loc, index } => {
                let origin = self.origin_loc_name(origin_loc);
                format!("?{origin}_{index}")
            }
        }
    }

    fn render_ty_callable(&self, ty_callable: &TyCallable) -> String {
        match ty_callable.kind {
            CallKind::Fun => {
                let params = format!("fn({})", self.render_list(&ty_callable.param_types, ", "));
                let ret_type = ty_callable.ret_type();
                if matches!(ret_type, Ty::Unit) {
                    params
                } else {
                    format!("{} -> {}", params, self.render(&ret_type))
                }
            }
            CallKind::Lambda => {
                let params = format!("|{}|", self.render_list(&ty_callable.param_types, ", "));
                let ret_type = ty_callable.ret_type();
                if matches!(ret_type, Ty::Unit) {
                    format!("{} -> ()", params)
                } else {
                    format!("{} -> {}", params, self.render(&ret_type))
                }
            }
        }
    }

    fn render_ty_adt(&self, ty_adt: &TyAdt) -> String {
        let item = ty_adt.adt_item_loc.to_ast::<ast::StructOrEnum>(self.db).unwrap();
        let item_fq_name = self.render_fq_item(item.map_into()).unwrap_or(anonymous());
        format!("{}{}", item_fq_name, self.render_type_args(&ty_adt.type_args))
    }

    fn render_ty_schema(&self, ty_adt: &TySchema) -> String {
        let item = ty_adt.schema_loc.to_ast::<ast::Schema>(self.db).unwrap();
        let item_fq_name = self.render_fq_item(item.map_into()).unwrap_or(anonymous());
        // let item_fq_name = item
        //     .fq_name(self.db)
        //     .map(|it| it.fq_identifier_text())
        //     .unwrap_or(anonymous());
        format!("{}{}", item_fq_name, self.render_type_args(&ty_adt.type_args))
    }

    fn render_type_args(&self, type_args: &Vec<Ty>) -> String {
        if type_args.is_empty() {
            return "".to_string();
        }
        format!("<{}>", self.render_list(type_args, ", "))
    }

    fn render_fq_item(&self, item: InFile<ast::Item>) -> Option<String> {
        let fq_name = item.fq_name(self.db)?;

        let Some(ctx_file_id) = self.context_file_id else {
            return Some(fq_name.fq_identifier_text());
        };

        let addr_name = fq_name.address().identifier_text();
        if matches!(addr_name.as_str(), "std" | "aptos_std") {
            return Some(fq_name.name());
        }

        let item_package_id = self.db.file_package_id(item.file_id);
        let context_package_id = self.db.file_package_id(ctx_file_id);
        if item_package_id == context_package_id {
            return Some(fq_name.name());
        }

        Some(fq_name.module_and_item_text())
    }

    fn origin_loc_name(&self, origin_loc: &SyntaxLoc) -> String {
        origin_loc
            .to_ast::<ast::TypeParam>(self.db)
            .and_then(|tp| tp.value.name())
            .map(|tp_name| tp_name.as_string())
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
