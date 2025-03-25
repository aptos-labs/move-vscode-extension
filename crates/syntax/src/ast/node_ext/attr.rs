use crate::{ast, AstNode};

impl ast::Attr {
    pub fn as_simple_atom(&self) -> Option<String> {
        let attr_item = self.attr_item()?;
        if attr_item.eq_token().is_some() {
            return None;
        }
        self.simple_name()
    }

    pub fn simple_name(&self) -> Option<String> {
        let path = self.attr_item()?.path()?;
        let segment = path.segment()?;
        if path.qualifier().is_some() {
            return None;
        }
        Some(segment.syntax().first_token()?.text().into())
    }

    pub fn path(&self) -> Option<ast::Path> {
        self.attr_item()?.path()
    }

    pub fn expr(&self) -> Option<ast::Expr> {
        self.attr_item()?.expr()
    }
}
