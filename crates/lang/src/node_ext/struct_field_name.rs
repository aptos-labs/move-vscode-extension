use syntax::ast;
use syntax::ast::HasName;
use crate::{AsName, Name};
use crate::nameres::path_kind::path_kind;
use crate::node_ext::PathLangExt;

pub trait StructFieldNameExt {
    fn field_name(&self) -> Option<Name>;
}

impl StructFieldNameExt for ast::StructPatField {
    fn field_name(&self) -> Option<Name> {
        if let Some(name_ref) = self.name_ref() {
            return Some(name_ref.as_name());
        }
        if let Some(ident_pat_name) = self.ident_pat().and_then(|it| it.name()) {
            return Some(ident_pat_name.as_name());
        }
        None
    }
}

impl StructFieldNameExt for ast::StructLitField {
    fn field_name(&self) -> Option<Name> {
        if let Some(name_ref) = self.name_ref() {
            return Some(name_ref.as_name());
        }
        let path = self.expr()?.path_expr()?.path()?;
        if path.coloncolon_token().is_none() {
            return path.name_ref_name();
        }
        None
    }
}