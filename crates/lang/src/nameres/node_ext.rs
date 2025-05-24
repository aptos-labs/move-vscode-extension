use crate::nameres::namespaces::Ns;
use crate::nameres::scope::{NamedItemsExt, ScopeEntry, ScopeEntryExt};
use crate::node_ext::item::ModuleItemExt;
use base_db::inputs::InternFileId;
use base_db::{SourceDatabase, source_db};
use std::iter;
use syntax::ast;
use syntax::ast::HasItems;
use syntax::files::{InFile, InFileExt};

pub trait ModuleResolutionExt {
    fn item_entries(&self) -> Vec<ScopeEntry>;
    fn importable_entries(&self) -> Vec<ScopeEntry>;
    fn related_module_specs(&self, db: &dyn SourceDatabase) -> Vec<InFile<ast::ModuleSpec>>;
    fn importable_entries_from_related(&self, db: &dyn SourceDatabase) -> Vec<ScopeEntry>;
}

impl ModuleResolutionExt for InFile<ast::Module> {
    fn item_entries(&self) -> Vec<ScopeEntry> {
        let (file_id, module) = self.unpack_ref();

        let mut entries = vec![];
        for member in module.named_items() {
            if let Some(struct_) = member.cast_into::<ast::Struct>() {
                if struct_.is_tuple_struct() {
                    if let Some(s_entry) = struct_.in_file(file_id).to_entry() {
                        entries.extend(vec![s_entry.clone(), s_entry.copy_with_ns(Ns::NAME)]);
                    }
                    continue;
                }
            }
            if let Some(entry) = member.in_file(self.file_id).to_entry() {
                entries.push(entry);
            }
        }

        entries
    }

    fn importable_entries(&self) -> Vec<ScopeEntry> {
        let mut entries = self.item_entries();
        entries.extend(self.clone().flat_map(|it| it.global_variables()).to_entries());
        entries
    }

    /// collects `spec MODULE {}` from all spec-related (NAME.move + NAME.spec.move) file ids
    #[tracing::instrument(level = "debug", skip_all)]
    fn related_module_specs(&self, db: &dyn SourceDatabase) -> Vec<InFile<ast::ModuleSpec>> {
        let related_file_ids =
            iter::once(self.file_id).chain(db.spec_related_files(self.file_id).data(db));
        let mut module_specs = vec![];
        for spec_related_file_id in related_file_ids {
            let source_file = source_db::parse(db, spec_related_file_id.intern(db)).tree();
            for module_spec in source_file.module_specs() {
                let module_spec = module_spec.in_file(spec_related_file_id);
                if module_spec.module(db).is_some_and(|item| &item == self) {
                    module_specs.push(module_spec);
                }
            }
        }
        module_specs
    }

    #[tracing::instrument(level = "debug", skip_all)]
    fn importable_entries_from_related(&self, db: &dyn SourceDatabase) -> Vec<ScopeEntry> {
        let mut entries = vec![];
        for related_module_spec in self.related_module_specs(db) {
            let module_spec_entries = related_module_spec
                .flat_map(|it| it.importable_items())
                .to_entries();
            entries.extend(module_spec_entries);
        }
        entries
    }
}

pub trait FileResolutionExt {
    fn importable_entries(&self) -> Vec<ScopeEntry>;
}

impl FileResolutionExt for InFile<ast::SourceFile> {
    fn importable_entries(&self) -> Vec<ScopeEntry> {
        let mut entries = vec![];
        let modules = self.clone().flat_map(|it| it.all_modules().collect());
        for module in modules {
            if let Some(module_entry) = module.clone().to_entry() {
                entries.push(module_entry);
            }
            entries.extend(module.importable_entries());
        }
        let module_specs = self.clone().flat_map(|it| it.module_specs().collect());
        for module_spec in module_specs {
            let items = module_spec.flat_map(|it| it.importable_items());
            entries.extend(items.to_entries());
        }
        entries
    }
}
