use crate::nameres::namespaces::{NAMES, NsSet, SCHEMAS, TYPES};
use stdx::itertools::Itertools;
use syntax::ast;
use syntax::ast::{AnyNamedElement, HasItems};

pub trait HasMembersList: HasItems {
    fn member_items_with_ns(&self) -> Vec<(Vec<AnyNamedElement>, NsSet)> {
        fn into_has_names(items: Vec<impl Into<AnyNamedElement>>) -> Vec<AnyNamedElement> {
            items.into_iter().map_into::<AnyNamedElement>().collect()
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
