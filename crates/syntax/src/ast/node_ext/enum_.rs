use crate::ast;

impl ast::Enum {
    pub fn variants(&self) -> Vec<ast::Variant> {
        self.variant_list()
            .map(|list| list.variants().collect())
            .unwrap_or_default()
    }
}
