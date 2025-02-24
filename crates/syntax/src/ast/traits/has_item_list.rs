use crate::ast::{support, AnyHasName, ItemList};
use crate::{ast, AstNode};
use std::iter;
use stdx::itertools::Itertools;

pub trait HasItemList: AstNode {
    #[inline]
    fn item_list(&self) -> Option<ItemList> {
        support::child(&self.syntax())
    }

    fn items(&self) -> Vec<ast::Item> {
        self.item_list()
            .map(|list| list.items().collect())
            .unwrap_or_default()
    }

    fn consts(&self) -> Vec<ast::Const> {
        self.items().into_iter().filter_map(|it| it.const_()).collect()
    }

    fn enums(&self) -> Vec<ast::Enum> {
        self.items().into_iter().filter_map(|it| it.enum_()).collect()
    }

    fn functions(&self) -> Vec<ast::Fun> {
        self.items().into_iter().filter_map(|it| it.fun()).collect()
    }

    fn structs(&self) -> Vec<ast::Struct> {
        self.items().into_iter().filter_map(|it| it.struct_()).collect()
    }

    fn use_items(&self) -> Vec<ast::UseItem> {
        self.items().into_iter().filter_map(|it| it.use_item()).collect()
    }

    fn use_specks(&self) -> Vec<ast::UseSpeck> {
        self.use_items()
            .into_iter()
            .filter_map(|i| i.use_speck())
            .flat_map(|use_speck| {
                if let Some(use_group) = use_speck.use_group() {
                    let mut v = vec![use_speck];
                    v.extend(use_group.use_specks());
                    v
                } else {
                    vec![use_speck]
                }
            })
            .collect()
    }

    fn spec_functions(&self) -> Vec<ast::SpecFun> {
        self.items().into_iter().filter_map(|it| it.spec_fun()).collect()
    }

    fn schemas(&self) -> Vec<ast::Schema> {
        self.items().into_iter().filter_map(|it| it.schema()).collect()
    }
}
