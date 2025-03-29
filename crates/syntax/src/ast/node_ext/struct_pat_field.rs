use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::{ast, AstNode};

impl ast::StructPatField {
    pub fn struct_pat(&self) -> ast::StructPat {
        self.syntax()
            .ancestor_of_type::<ast::StructPat>(true)
            .expect("required by parser")
    }

    pub fn kind(&self) -> PatFieldKind {
        let ident_pat = self.ident_pat();
        if let (Some(pat), Some(name_ref)) = (self.pat(), self.name_ref()) {
            return PatFieldKind::Full { name_ref, pat };
        }
        if let Some(ident_pat) = self.ident_pat() {
            return PatFieldKind::Shorthand { ident_pat };
        }
        if let Some(rest_pat) = self.rest_pat() {
            return PatFieldKind::Rest;
        }
        PatFieldKind::Invalid
    }

    pub fn field_name(&self) -> Option<String> {
        match self.kind() {
            PatFieldKind::Full { name_ref, .. } => Some(name_ref.as_string()),
            PatFieldKind::Shorthand { ident_pat } => Some(ident_pat.as_string()),
            _ => None,
        }
    }
}

pub enum PatFieldKind {
    Full { name_ref: ast::NameRef, pat: ast::Pat },
    Shorthand { ident_pat: ast::IdentPat },
    Rest,
    Invalid,
}
