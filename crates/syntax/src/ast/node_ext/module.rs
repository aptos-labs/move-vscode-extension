use crate::ast;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::ast::traits::into_named_elements;
use crate::ast::HasItems;

impl ast::Module {
    pub fn parent_address_def(&self) -> Option<ast::AddressDef> {
        self.syntax.parent_of_type::<ast::AddressDef>()
    }

    pub fn self_or_parent_address_ref(&self) -> Option<ast::AddressRef> {
        self.address_ref()
            .or_else(|| self.parent_address_def().and_then(|def| def.address_ref()))
    }

    pub fn friend_decls(&self) -> Vec<ast::Friend> {
        self.items()
            .into_iter()
            .filter_map(|item| item.friend())
            .collect()
    }

    pub fn named_items(&self) -> Vec<ast::NamedElement> {
        let mut items: Vec<ast::NamedElement> = vec![];
        // consts
        items.extend(into_named_elements(self.consts()));

        // types
        items.extend(into_named_elements(self.enums()));
        items.extend(into_named_elements(self.structs()));
        items.extend(into_named_elements(self.schemas()));

        // callables
        items.extend(into_named_elements(self.non_test_functions()));

        // spec callables
        items.extend(into_named_elements(self.spec_functions()));
        items.extend(into_named_elements(self.spec_inline_functions()));
        // entries.extend(
        //     module
        //         .spec_inline_functions()
        //         .wrapped_in_file(self.file_id)
        //         .to_entries(),
        // );
        items
    }

    pub fn verifiable_items(&self) -> Vec<ast::NamedElement> {
        let mut items: Vec<ast::NamedElement> = vec![];
        items.extend(self.non_test_functions().into_iter().map(|it| it.into()));
        items.extend(self.structs().into_iter().map(|it| it.into()));
        items.extend(self.enums().into_iter().map(|it| it.into()));
        items
    }
}
