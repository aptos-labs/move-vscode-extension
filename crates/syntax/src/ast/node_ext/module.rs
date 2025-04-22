use crate::ast;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
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

    pub fn verifiable_items(&self) -> Vec<ast::AnyNamedElement> {
        let mut items: Vec<ast::AnyNamedElement> = vec![];
        items.extend(self.non_test_functions().into_iter().map(|it| it.into()));
        items.extend(self.structs().into_iter().map(|it| it.into()));
        items.extend(self.enums().into_iter().map(|it| it.into()));
        items
    }
}
