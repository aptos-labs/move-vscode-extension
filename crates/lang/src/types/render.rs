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
use std::fmt;
use stdx::itertools::Itertools;
use syntax::ast;
use syntax::files::InFile;
use vfs::FileId;

pub trait HirWrite: fmt::Write {
    fn start_location_link(&mut self, _named_item: InFile<ast::NamedElement>) -> Option<()> {
        Some(())
    }
    fn end_location_link(&mut self) {}
}

// String will ignore link metadata
impl HirWrite for String {}

// `core::Formatter` will ignore metadata
impl HirWrite for fmt::Formatter<'_> {}

pub struct TypeRenderer<'db> {
    db: &'db dyn SourceDatabase,
    current_file_id: Option<FileId>,
    sink: &'db mut dyn HirWrite,
}

impl<'db> TypeRenderer<'db> {
    pub fn new(
        db: &'db dyn SourceDatabase,
        context: Option<FileId>,
        sink: &'db mut dyn HirWrite,
    ) -> Self {
        TypeRenderer {
            db,
            current_file_id: context,
            sink,
        }
    }

    pub fn write_str(&mut self, s: &str) -> fmt::Result {
        self.sink.write_str(s)
    }

    pub fn render(&mut self, ty: &Ty) -> anyhow::Result<()> {
        match ty {
            Ty::Seq(ty_seq) => {
                let type_name = match ty_seq {
                    TySequence::Vector(_) => "vector",
                    TySequence::Range(_) => "range",
                };
                self.write_str(type_name)?;
                self.render_type_args(&vec![ty_seq.item()])?;
            }
            Ty::Adt(ty_adt) => self.render_ty_adt(ty_adt)?,
            Ty::Schema(ty_schema) => self.render_ty_schema(ty_schema)?,
            Ty::Callable(ty_callable) => self.render_ty_callable(ty_callable)?,
            Ty::Reference(ty_ref) => {
                let prefix = if ty_ref.is_mut() { "&mut " } else { "&" };
                self.write_str(prefix)?;
                self.render(&ty_ref.referenced())?;
            }
            Ty::Tuple(ty_tuple) => {
                self.render_type_list("(", &ty_tuple.types, ")")?;
            }

            Ty::TypeParam(ty_tp) => self.render_type_param(ty_tp)?,
            Ty::Infer(ty_infer) => match ty_infer {
                TyInfer::Var(ty_var) => self.render_ty_var(ty_var)?,
                TyInfer::IntVar(_) => self.write_str("?integer")?,
            },

            Ty::Bool => self.write_str("bool")?,
            Ty::Signer => self.write_str("signer")?,
            Ty::Address => self.write_str("address")?,
            Ty::Integer(kind) => self.write_str(&kind.to_string())?,
            Ty::Num => self.write_str("num")?,
            Ty::Bv => self.write_str("bv")?,

            Ty::Unit => self.write_str("()")?,
            Ty::Unknown => self.sink.write_str(UNKNOWN)?,
            Ty::Never => self.write_str(NEVER)?,
        }
        Ok(())
    }

    fn render_type_param(&mut self, type_param: &TyTypeParameter) -> fmt::Result {
        self.write_str(&self.origin_loc_name(&type_param.origin_loc))
    }

    fn render_ty_var(&mut self, ty_var: &TyVar) -> fmt::Result {
        match &ty_var.kind {
            TyVarKind::Anonymous(index) => write!(self.sink, "?_{index}"),
            TyVarKind::WithOrigin { origin_loc, index } => {
                let origin = self.origin_loc_name(origin_loc);
                write!(self.sink, "?{origin}_{index}")
            }
        }
    }

    fn render_ty_callable(&mut self, ty_callable: &TyCallable) -> anyhow::Result<()> {
        match ty_callable.kind {
            CallKind::Fun => {
                self.render_type_list("fn(", &ty_callable.param_types, ")")?;
                let ret_type = ty_callable.ret_type();
                if !matches!(ret_type, Ty::Unit) {
                    self.write_str(" -> ")?;
                    self.render(&ret_type)?;
                }
            }
            CallKind::Lambda => {
                self.render_type_list("|", &ty_callable.param_types, "|")?;
                let ret_type = ty_callable.ret_type();
                self.write_str(" -> ")?;
                self.render(&ret_type)?;
            }
        }
        Ok(())
    }

    fn render_ty_adt(&mut self, ty_adt: &TyAdt) -> anyhow::Result<()> {
        let item = ty_adt.adt_item_loc.to_ast::<ast::StructOrEnum>(self.db).unwrap();
        self.render_fq_name(item.map_into())?;
        self.render_type_args(&ty_adt.type_args)?;
        Ok(())
    }

    fn render_ty_schema(&mut self, ty_adt: &TySchema) -> anyhow::Result<()> {
        let item = ty_adt.schema_loc.to_ast::<ast::Schema>(self.db).unwrap();
        self.render_fq_name(item.map_into())?;
        self.render_type_args(&ty_adt.type_args)?;
        Ok(())
    }

    fn render_type_args(&mut self, type_args: &Vec<Ty>) -> anyhow::Result<()> {
        if type_args.is_empty() {
            return Ok(());
        }
        self.render_type_list("<", type_args, ">")
    }

    fn render_type_list(&mut self, prefix: &str, tys: &Vec<Ty>, suffix: &str) -> anyhow::Result<()> {
        self.write_str(prefix)?;
        for (i, ty) in tys.iter().enumerate() {
            self.render(&ty)?;
            if i != tys.len() - 1 {
                self.write_str(", ")?;
            }
        }
        self.write_str(suffix)?;
        Ok(())
    }

    fn render_fq_name(&mut self, item: InFile<ast::NamedElement>) -> anyhow::Result<()> {
        let Some(fq_name) = item.fq_name(self.db) else {
            self.write_str(UNRESOLVED)?;
            return Ok(());
        };

        self.sink.start_location_link(item.clone());

        let Some(ctx_file_id) = self.current_file_id else {
            self.write_str(&fq_name.fq_identifier_text())?;
            return Ok(());
        };

        let addr_name = fq_name.address().identifier_text();
        if matches!(addr_name.as_str(), "std" | "aptos_std") {
            self.write_str(&fq_name.name())?;
            return Ok(());
        }

        let item_package_id = self.db.file_package_id(item.file_id);
        let context_package_id = self.db.file_package_id(ctx_file_id);
        if item_package_id == context_package_id {
            self.write_str(&fq_name.name())?;
            return Ok(());
        }
        self.write_str(&fq_name.module_and_item_text())?;

        self.sink.end_location_link();

        Ok(())
    }

    fn origin_loc_name(&self, origin_loc: &SyntaxLoc) -> String {
        origin_loc
            .to_ast::<ast::TypeParam>(self.db)
            .and_then(|tp| tp.value.name())
            .map(|tp_name| tp_name.as_string())
            .unwrap_or(unresolved())
    }
}

const UNKNOWN: &str = "<unknown>";
const NEVER: &str = "<never>";
const UNRESOLVED: &str = "<anonymous>";

fn unresolved() -> String {
    UNRESOLVED.to_string()
}
