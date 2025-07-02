use crate::ast;
use crate::ast::HasItems;
use crate::ast::traits::into_named_elements;

impl ast::ModuleSpec {
    pub fn importable_items(&self) -> Vec<ast::NamedElement> {
        let mut items: Vec<ast::NamedElement> = vec![];

        items.extend(into_named_elements(self.schemas()));
        items.extend(into_named_elements(self.spec_functions()));
        items.extend(into_named_elements(self.spec_inline_functions()));
        items.extend(into_named_elements(self.global_variables()));

        items
    }
}
