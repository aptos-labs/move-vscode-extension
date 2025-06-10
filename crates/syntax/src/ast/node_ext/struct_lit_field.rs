use crate::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::{ast, AstNode};

impl ast::StructLitField {
    pub fn for_field_name(field_name: &ast::NameRef) -> Option<ast::StructLitField> {
        let candidate = Self::for_name_ref(field_name)?;
        if candidate.field_name().as_ref() == Some(field_name) {
            Some(candidate)
        } else {
            None
        }
    }

    pub fn for_name_ref(name_ref: &ast::NameRef) -> Option<ast::StructLitField> {
        let struct_field = name_ref.syntax.parent_of_type::<ast::StructLitField>();
        if struct_field.is_some() {
            return struct_field;
        }
        // shorthand
        let path = name_ref
            .syntax
            .parent_of_type::<ast::PathSegment>()?
            .parent_path();
        Self::for_shorthand_path(&path)
    }

    pub fn for_shorthand_path(path: &ast::Path) -> Option<ast::StructLitField> {
        let path_expr = path.path_expr()?;
        path_expr.syntax.parent_of_type::<ast::StructLitField>()
    }

    pub fn shorthand_path_expr(&self) -> Option<ast::PathExpr> {
        let path_expr = self.expr()?.path_expr()?;
        Some(path_expr)
    }

    pub fn shorthand_path(&self) -> Option<ast::Path> {
        let path_expr = self.expr()?.path_expr()?;
        Some(path_expr.path())
    }

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

    // pub fn field_name(&self) -> Option<String> {
    //     if let Some(name_ref) = self.name_ref() {
    //         return Some(name_ref.as_string());
    //     }
    //     let path = self.expr()?.path_expr()?.path();
    //     if path.coloncolon_token().is_none() {
    //         return path.reference_name();
    //     }
    //     None
    // }
}
