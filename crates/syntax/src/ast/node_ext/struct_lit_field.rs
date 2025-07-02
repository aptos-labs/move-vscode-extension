use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::{AstNode, ast};

#[derive(Debug)]
pub enum StructLitFieldKind {
    Full {
        struct_field: ast::StructLitField,
        name_ref: ast::NameRef,
        expr: Option<ast::Expr>,
    },
    Shorthand {
        struct_field: ast::StructLitField,
        path: ast::Path,
    },
}

impl ast::NameRef {
    pub fn try_into_struct_lit_field(&self) -> Option<StructLitFieldKind> {
        let struct_field = self.syntax.parent_of_type::<ast::StructLitField>();
        if let Some(struct_field) = struct_field {
            let expr = struct_field.expr().clone();
            return Some(StructLitFieldKind::Full {
                struct_field,
                name_ref: self.clone(),
                expr,
            });
        }
        // might also be a field shorthand
        let name_ref_path = self.syntax.parent_of_type::<ast::PathSegment>()?.parent_path();
        let path_expr = name_ref_path.path_expr()?;
        if let Some(struct_field) = path_expr.syntax.parent_of_type::<ast::StructLitField>() {
            if struct_field.name_ref().is_none() {
                return Some(StructLitFieldKind::Shorthand {
                    struct_field,
                    path: name_ref_path,
                });
            }
        }
        None
    }
}

impl ast::StructLitField {
    /// Deals with field init shorthand
    pub fn field_name(&self) -> Option<ast::NameRef> {
        if let Some(name_ref) = self.name_ref() {
            return Some(name_ref);
        }
        let path = self.expr()?.path_expr()?.path();
        let segment = path.segment()?;
        let name_ref = segment.name_ref()?;
        if path.qualifier().is_none() {
            return Some(name_ref);
        }
        None
    }

    pub fn struct_lit(&self) -> ast::StructLit {
        self.syntax()
            .ancestor_of_type::<ast::StructLit>(true)
            .expect("required by parser")
    }
}
