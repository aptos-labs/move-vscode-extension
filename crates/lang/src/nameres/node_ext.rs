use crate::files::{InFileExt, InFileVecExt};
use crate::nameres::namespaces::Ns;
use crate::nameres::scope::{NamedItemsExt, ScopeEntry, ScopeEntryExt};
use crate::InFile;
use syntax::ast;
use syntax::ast::HasItems;

pub trait ModuleResolutionExt {
    fn member_entries(&self) -> Vec<ScopeEntry>;
}

impl ModuleResolutionExt for InFile<ast::Module> {
    fn member_entries(&self) -> Vec<ScopeEntry> {
        let mut entries = vec![];
        let module = &self.value;
        // consts
        // entries.extend(self.consts().to_entries());
        entries.extend(module.consts().wrapped_in_file(self.file_id).to_entries());

        // types
        entries.extend(module.enums().wrapped_in_file(self.file_id).to_entries());
        entries.extend(module.structs().wrapped_in_file(self.file_id).to_entries());
        entries.extend(module.schemas().wrapped_in_file(self.file_id).to_entries());

        // callables
        // todo: filter by #[test]
        entries.extend(module.functions().wrapped_in_file(self.file_id).to_entries());
        entries.extend(module.tuple_structs().into_iter().filter_map(|s| {
            s.in_file(self.file_id)
                .to_entry()
                .map(|entry| entry.copy_with_ns(Ns::NAME))
        }));

        // spec callables
        entries.extend(module.spec_functions().wrapped_in_file(self.file_id).to_entries());
        entries.extend(
            module
                .spec_inline_functions()
                .wrapped_in_file(self.file_id)
                .to_entries(),
        );

        entries
    }
}
