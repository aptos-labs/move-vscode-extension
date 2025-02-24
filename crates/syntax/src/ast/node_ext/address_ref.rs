use crate::ast;

impl ast::ValueAddress {
    pub fn address_text(&self) -> String {
        self.int_number_token().unwrap().text().to_string()
    }
}
