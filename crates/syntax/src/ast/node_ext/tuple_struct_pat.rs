use crate::ast;

impl ast::TupleStructPat {
    pub fn has_rest_pat(&self) -> bool {
        self.fields().into_iter().any(|it| it.rest_pat().is_some())
    }
}
