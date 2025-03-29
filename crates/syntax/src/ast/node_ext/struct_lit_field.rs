use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::{ast, AstNode};

impl ast::StructLitField {
    pub fn struct_lit(&self) -> ast::StructLit {
        self.syntax()
            .ancestor_of_type::<ast::StructLit>(true)
            .expect("required by parser")
    }

    pub fn field_name(&self) -> Option<String> {
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
