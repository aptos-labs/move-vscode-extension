use crate::ast;
use crate::ast::traits::into_named_elements;
use crate::ast::HasItems;

impl ast::ModuleSpec {
    pub fn importable_items(&self) -> Vec<ast::AnyNamedElement> {
        let mut items: Vec<ast::AnyNamedElement> = vec![];

        items.extend(into_named_elements(self.schemas()));
        items.extend(into_named_elements(self.spec_functions()));
        items.extend(into_named_elements(self.spec_inline_functions()));
        items.extend(into_named_elements(self.global_variables()));

        items
    }
}
