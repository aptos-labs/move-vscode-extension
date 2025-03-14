use crate::{ast, AstNode};
use crate::ast::HasItemList;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;

impl ast::Module {
    pub fn parent_address_def(&self) -> Option<ast::AddressDef> {
        self.syntax.parent_of_type::<ast::AddressDef>()
    }

    pub fn self_or_parent_address_ref(&self) -> Option<ast::AddressRef> {
        self.address_ref()
            .or_else(|| self.parent_address_def().and_then(|def| def.address_ref()))
    }

    pub fn friend_decls(&self) -> Vec<ast::Friend> {
        self.items().into_iter().filter_map(|item| item.friend()).collect()
    }
}
