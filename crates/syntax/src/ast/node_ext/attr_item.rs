use crate::ast;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;

impl ast::AttrItem {
    pub fn attr(&self) -> Option<ast::Attr> {
        self.syntax.parent_of_type::<ast::Attr>()
    }

    pub fn parent_attr_item(&self) -> Option<ast::AttrItem> {
        self.parent_attr_item_list()?
            .syntax
            .parent_of_type::<ast::AttrItem>()
    }

    pub fn parent_attr_item_list(&self) -> Option<ast::AttrItemList> {
        self.syntax.parent_of_type::<ast::AttrItemList>()
    }

    pub fn is_name_only(&self) -> bool {
        self.initializer().is_none() && self.attr_item_list().is_none()
    }

    pub fn no_qual_name(&self) -> Option<String> {
        let path = self.path()?;
        if !path.is_local() {
            return None;
        }
        path.reference_name()
    }

    pub fn is_abort_code(&self) -> bool {
        if self.no_qual_name().is_none_or(|it| it != "abort_code") {
            return false;
        }
        // confirm that the position is correct
        if let Some(parent_attr_item) = self.parent_attr_item()
            && let Some(attr) = parent_attr_item.attr()
        {
            return attr
                .single_attr_item_name()
                .is_some_and(|it| it.as_str() == "test");
        };
        false
    }
}
