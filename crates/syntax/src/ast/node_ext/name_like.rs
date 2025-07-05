use crate::ast;

impl ast::NameLike {
    pub fn as_string(&self) -> String {
        match self {
            ast::NameLike::NameRef(name_ref) => name_ref.as_string(),
            ast::NameLike::Name(name) => name.as_string(),
        }
    }
}
