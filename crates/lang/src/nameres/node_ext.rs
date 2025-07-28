// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::nameres::namespaces::Ns;
use crate::nameres::scope::{NamedItemsExt, NamedItemsInFileExt, ScopeEntry, ScopeEntryExt};
use crate::node_ext::item::ModuleItemExt;
use base_db::inputs::InternFileId;
use base_db::{SourceDatabase, source_db};
use syntax::ast;
use syntax::ast::HasItems;
use syntax::files::{InFile, InFileExt};

pub trait ModuleResolutionExt {
    fn module(&self) -> InFile<&ast::Module>;

    fn item_entries(&self) -> Vec<ScopeEntry> {
        let (file_id, module) = self.module().unpack();

        let module_named_items = module.named_items();

        let mut entries = Vec::with_capacity(module_named_items.len());
        for member in module_named_items {
            if let Some(struct_) = member.clone().struct_() {
                if struct_.is_tuple_struct() {
                    if let Some(s_entry) = struct_.in_file(file_id).to_entry() {
                        entries.extend(vec![s_entry.clone(), s_entry.copy_with_ns(Ns::NAME)]);
                    }
                    continue;
                }
            }
            if let Some(entry) = member.in_file(file_id).to_entry() {
                entries.push(entry);
            }
        }

        entries
    }

    fn importable_entries(&self) -> Vec<ScopeEntry> {
        let mut entries = self.item_entries();

        let (file_id, module) = self.module().unpack();
        entries.extend(module.global_variables().to_entries(file_id));

        entries
    }

    fn related_module_specs(&self, db: &dyn SourceDatabase) -> Vec<InFile<ast::ModuleSpec>> {
        let spec_file_set = source_db::spec_union_file_set(db, self.module().file_id);
        let mut module_specs = Vec::with_capacity(spec_file_set.len());
        for spec_related_file_id in spec_file_set {
            let source_file = source_db::parse(db, spec_related_file_id.intern(db)).tree();
            for module_spec in source_file.module_specs() {
                let module_spec = module_spec.in_file(spec_related_file_id);
                if module_spec
                    .module(db)
                    .is_some_and(|item| item.as_ref() == self.module())
                {
                    module_specs.push(module_spec);
                }
            }
        }
        module_specs
    }

    fn importable_entries_from_related(&self, db: &dyn SourceDatabase) -> Vec<ScopeEntry> {
        let mut entries = vec![];
        for related_module_spec in self.related_module_specs(db) {
            let spec_importable_items = related_module_spec.value.importable_items();
            entries.reserve(spec_importable_items.len());
            for importable_item in spec_importable_items {
                if let Some(entry) = importable_item.in_file(related_module_spec.file_id).to_entry() {
                    entries.push(entry);
                }
            }
        }
        entries
    }
}

impl ModuleResolutionExt for InFile<ast::Module> {
    fn module(&self) -> InFile<&ast::Module> {
        self.as_ref()
    }
}
