use crate::loc::SyntaxLocExt;
use crate::types::ty::Ty;
use crate::{loc, InFile};
use std::collections::HashMap;
use syntax::ast;
use vfs::FileId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InferenceResult {
    pub file_id: FileId,
    pub expr_types: HashMap<loc::SyntaxLoc, Ty>,
}

impl InferenceResult {
    pub fn get_expr_type(&self, expr: ast::Expr) -> Option<Ty> {
        let expr_loc = InFile::new(self.file_id, expr).loc();
        self.expr_types.get(&expr_loc).map(|it| it.to_owned())
    }
}
