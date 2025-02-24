use crate::nameres::namespaces::{NsSet, NAMES, SCHEMAS, TYPES};
use stdx::itertools::Itertools;
use syntax::ast;
use syntax::ast::{AnyHasName, HasItemList};

pub trait HasMembersList: HasItemList {
    fn member_items_with_ns(&self) -> Vec<(Vec<AnyHasName>, NsSet)> {
        fn into_has_names(items: Vec<impl Into<AnyHasName>>) -> Vec<AnyHasName> {
            items.into_iter().map_into::<AnyHasName>().collect()
        }
        let mut v = vec![];
        v.push((into_has_names(self.consts()), NAMES));
        v.push((into_has_names(self.functions()), NAMES));
        v.push((into_has_names(self.structs()), TYPES));
        v.push((into_has_names(self.enums()), TYPES));
        v.push((into_has_names(self.spec_functions()), NAMES));
        v.push((into_has_names(self.schemas()), SCHEMAS));
        v
    }
}

impl HasMembersList for ast::Module {}
