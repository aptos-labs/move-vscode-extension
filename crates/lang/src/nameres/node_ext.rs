use crate::nameres::namespaces::NAMES;
use crate::nameres::scope::{NamedItemsExt, ScopeEntry, ScopeEntryExt};
use syntax::ast;
use syntax::ast::HasItemList;

pub trait ModuleResolutionExt {
    fn member_entries(&self) -> Vec<ScopeEntry>;
}

impl ModuleResolutionExt for ast::Module {
    fn member_entries(&self) -> Vec<ScopeEntry> {
        let mut entries = vec![];
        // consts
        entries.extend(self.consts().to_entries());

        // types
        entries.extend(self.enums().to_entries());
        entries.extend(self.structs().to_entries());
        entries.extend(self.schemas().to_entries());

        // callables
        // todo: filter by #[test]
        entries.extend(self.functions().to_entries());
        entries.extend(
            self.tuple_structs()
                .into_iter()
                .filter_map(|s| s.to_entry().map(|entry| entry.copy_with_ns(NAMES))),
        );

        // spec callables
        entries.extend(self.spec_functions().to_entries());
        entries.extend(self.spec_inline_functions().to_entries());

        entries
    }
}
