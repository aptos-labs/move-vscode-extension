// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::symbol_index::collector::SymbolCollector;
use base_db::SourceDatabase;
use base_db::package_root::PackageId;
use lang::hir_db;
use lang::loc::SyntaxLoc;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use syntax::ast;

pub fn world_symbols_in_package(db: &dyn SourceDatabase, package_id: PackageId) -> Arc<SymbolIndex> {
    let _p = tracing::info_span!("library_symbols").entered();

    let mut symbol_collector = SymbolCollector::new(db);

    let modules = hir_db::modules_for_package_id(db, package_id)
        .iter()
        .filter_map(|it| it.to_ast::<ast::Module>(db))
        .collect::<Vec<_>>();
    for module in modules {
        symbol_collector.collect_module(module);
    }

    Arc::new(SymbolIndex::new(symbol_collector.finish()))
}

#[derive(Default)]
pub struct SymbolIndex {
    pub(crate) symbols: Box<[FileSymbol]>,
    pub(crate) map: fst::Map<Vec<u8>>,
}

impl fmt::Debug for SymbolIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SymbolIndex")
            .field("n_symbols", &self.symbols.len())
            .finish()
    }
}

impl PartialEq for SymbolIndex {
    fn eq(&self, other: &SymbolIndex) -> bool {
        self.symbols == other.symbols
    }
}

impl Eq for SymbolIndex {}

impl Hash for SymbolIndex {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.symbols.hash(hasher)
    }
}

impl SymbolIndex {
    fn new(mut symbols: Box<[FileSymbol]>) -> SymbolIndex {
        fn cmp(lhs: &FileSymbol, rhs: &FileSymbol) -> Ordering {
            let lhs_chars = lhs.name.as_str().chars().map(|c| c.to_ascii_lowercase());
            let rhs_chars = rhs.name.as_str().chars().map(|c| c.to_ascii_lowercase());
            lhs_chars.cmp(rhs_chars)
        }

        symbols.sort_by(cmp);

        let mut builder = fst::MapBuilder::memory();

        let mut last_batch_start = 0;

        for idx in 0..symbols.len() {
            if let Some(next_symbol) = symbols.get(idx + 1) {
                if cmp(&symbols[last_batch_start], next_symbol) == Ordering::Equal {
                    continue;
                }
            }

            let start = last_batch_start;
            let end = idx + 1;
            last_batch_start = end;

            let key = symbols[start].name.as_str().to_ascii_lowercase();
            let value = SymbolIndex::range_to_map_value(start, end);

            builder.insert(key, value).unwrap();
        }

        let map = builder
            .into_inner()
            .and_then(|mut buf| {
                fst::Map::new({
                    buf.shrink_to_fit();
                    buf
                })
            })
            .unwrap();
        SymbolIndex { symbols, map }
    }

    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    pub fn memory_size(&self) -> usize {
        self.map.as_fst().size() + self.symbols.len() * size_of::<FileSymbol>()
    }

    fn range_to_map_value(start: usize, end: usize) -> u64 {
        debug_assert![start <= (u32::MAX as usize)];
        debug_assert![end <= (u32::MAX as usize)];

        ((start as u64) << 32) | end as u64
    }

    pub(crate) fn map_value_to_range(value: u64) -> (usize, usize) {
        let end = value as u32 as usize;
        let start = (value >> 32) as usize;
        (start, end)
    }
}

/// The actual data that is stored in the index. It should be as compact as
/// possible.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileSymbol {
    pub name: String,
    pub syntax_loc: SyntaxLoc,
    pub container_name: Option<String>,
    // /// Whether this symbol is a doc alias for the original symbol.
    // pub is_alias: bool,
    // pub is_import: bool,
    // pub do_not_complete: Complete,
}
