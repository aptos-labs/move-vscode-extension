use crate::{ast, match_ast, AstNode, SyntaxNode};

impl ast::ReferenceElement {
    // pub fn method_or_path(&self) -> Option<ast::MethodOrPath> {
    //     match self {
    //         ast::ReferenceElement::MethodCallExpr(method_call) => method_call.clone().into(),
    //         ast::ReferenceElement::Path(path) => path.clone().into(),
    //         _ => None,
    //     }
    // }

    pub fn reference_node(&self) -> Option<SyntaxNode> {
        match_ast! {
            match (self.syntax()) {
                ast::Path(it) => it.segment().map(|n| n.syntax),
                ast::MethodCallExpr(it) => it.name_ref().map(|n| n.syntax),
                ast::DotExpr(it) => it.name_ref().map(|n| n.syntax),
                ast::StructLitField(it) => {
                    it.name_ref().map(|it| it.syntax)
                },
                ast::StructPatField(it) => {
                    it.name_ref().map(|it| it.syntax)
                },
                _ => None,
            }
        }
    }

    pub fn reference_name(&self) -> Option<String> {
        match_ast! {
            match (self.syntax()) {
                ast::Path(it) => it.reference_name(),
                ast::MethodCallExpr(it) => Some(it.reference_name()),
                ast::DotExpr(it) => Some(it.name_ref()?.as_string()),
                ast::StructLitField(it) => {
                    it.name_ref().map(|it| it.as_string())
                },
                ast::StructPatField(it) => {
                    it.name_ref().map(|it| it.as_string())
                },
                _ => None,
            }
        }
    }
}
