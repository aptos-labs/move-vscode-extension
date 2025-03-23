use crate::ast;

impl ast::NameRef {
    // pub fn as_tuple_field(&self) -> Option<usize> {
    //     self.index_string().and_then(|it| it.parse().ok())
    // }

    // pub fn token_kind(&self) -> SyntaxKind {
    //     self.syntax()
    //         .first_token()
    //         .map_or(SyntaxKind::ERROR, |it| it.kind())
    // }

    pub fn as_string(&self) -> String {
        self.ident_token().text().to_string()
    }

    // pub fn index_string(&self) -> Option<String> {
    //     self.int_number_token().map(|it| it.text().to_string())
    // }
}
