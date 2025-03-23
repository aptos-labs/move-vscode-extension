use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::ast::NamedElement;
use crate::{ast, AstNode};

impl ast::StructPatField {
    pub fn struct_pat(&self) -> ast::StructPat {
        self.syntax()
            .ancestor_of_type::<ast::StructPat>(true)
            .expect("required by parser")
    }

    pub fn field_name(&self) -> Option<String> {
        if let Some(name_ref) = self.name_ref() {
            return Some(name_ref.as_string());
        }
        if let Some(ident_pat) = self.ident_pat() {
            return ident_pat.name().map(|it| it.as_string());
        }
        None
    }
}

impl ast::StructLitField {
    pub fn struct_lit(&self) -> ast::StructLit {
        self.syntax()
            .ancestor_of_type::<ast::StructLit>(true)
            .expect("required by parser")
    }
}
