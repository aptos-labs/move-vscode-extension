// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::symbol_index::sym_db::FileSymbol;
use base_db::SourceDatabase;
use indexmap::IndexSet;
use lang::loc::SyntaxLocFileExt;
use syntax::ast;
use syntax::files::InFile;

pub struct SymbolCollector<'a> {
    db: &'a dyn SourceDatabase,
    symbols: IndexSet<FileSymbol>,
}

impl<'a> SymbolCollector<'a> {
    pub fn new(db: &'a dyn SourceDatabase) -> Self {
        SymbolCollector {
            db,
            symbols: Default::default(),
        }
    }

    pub fn new_module(db: &dyn SourceDatabase, module: InFile<ast::Module>) -> Box<[FileSymbol]> {
        let mut symbol_collector = SymbolCollector::new(db);
        symbol_collector.collect_module(module);
        symbol_collector.finish()
    }

    pub fn finish(self) -> Box<[FileSymbol]> {
        self.symbols.into_iter().collect()
    }

    // fn do_work(&mut self, work: SymbolCollectorWork) {
    //     let _p = tracing::info_span!("SymbolCollector::do_work", ?work).entered();
    //     tracing::info!(?work, "SymbolCollector::do_work");
    //     self.db.unwind_if_revision_cancelled();
    //
    //     // let parent_name = work.parent.map(|name| name.as_str().to_smolstr());
    //     // self.with_container_name(parent_name, |s| s.collect_from_module(work.module_id));
    // }

    // fn with_container_name(&mut self, container_name: Option<String>, f: impl FnOnce(&mut Self)) {
    //     if let Some(container_name) = container_name {
    //         let prev = self.current_container_name.replace(container_name);
    //         f(self);
    //         self.current_container_name = prev;
    //     } else {
    //         f(self);
    //     }
    // }

    pub(crate) fn collect_module(&mut self, module: InFile<ast::Module>) -> Option<()> {
        let module_name = module.value.name()?.as_string();
        let named_items = module.flat_map(|it| it.named_items());
        for named_item in named_items {
            self.collect_named_item(Some(module_name.clone()), named_item);
        }
        Some(())
    }

    fn collect_named_item(
        &mut self,
        container_name: Option<String>,
        named_item: InFile<ast::NamedElement>,
    ) -> Option<()> {
        self.db.unwind_if_revision_cancelled();
        let item_name = named_item.value.name()?.as_string();
        if let Some(struct_) = named_item.cast_into_ref::<ast::Struct>() {
            // collect fields
            for named_field in struct_.flat_map(|it| it.named_fields()) {
                self.collect_named_item(Some(item_name.clone()), named_field.map_into());
            }
        }
        if let Some(enum_) = named_item.cast_into_ref::<ast::Enum>() {
            // collect variants
            // collect fields for variants
            // let enum_name = enum_.value.name()?.as_string();
            for variant in enum_.flat_map(|it| it.variants()) {
                self.collect_enum_variant(item_name.clone(), variant);
            }
        }
        self.symbols.insert(FileSymbol {
            name: item_name,
            syntax_loc: named_item.loc(),
            container_name,
        });
        Some(())
    }

    fn collect_enum_variant(&mut self, enum_name: String, variant: InFile<ast::Variant>) -> Option<()> {
        let variant_name = variant.value.name()?.as_string();

        self.collect_named_item(Some(enum_name), variant.clone().map_into());

        for named_field in variant.flat_map(|it| it.named_fields()) {
            self.collect_named_item(Some(variant_name.clone()), named_field.map_into());
        }
        Some(())
    }
}
