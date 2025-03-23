use crate::node_ext::PathLangExt;
use syntax::ast;
use syntax::ast::NamedElement;

pub trait StructFieldNameExt {
    fn field_name(&self) -> Option<String>;
}

impl StructFieldNameExt for ast::StructPatField {
    fn field_name(&self) -> Option<String> {
        if let Some(name_ref) = self.name_ref() {
            return Some(name_ref.as_string());
        }
        if let Some(ident_pat_name) = self.ident_pat().and_then(|it| it.name()) {
            return Some(ident_pat_name.as_string());
        }
        None
    }
}

impl StructFieldNameExt for ast::StructLitField {
    fn field_name(&self) -> Option<String> {
        if let Some(name_ref) = self.name_ref() {
            return Some(name_ref.as_string());
        }
        let path = self.expr()?.path_expr()?.path();
        if path.coloncolon_token().is_none() {
            return path.reference_name();
        }
        None
    }
}
