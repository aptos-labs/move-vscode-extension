// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

//! This module handles fuzzy-searching of functions, structs and other symbols
//! by name across the whole workspace and dependencies.
//!
//! It works by building an incrementally-updated text-search index of all
//! symbols. The backbone of the index is the **awesome** `fst` crate by
//! @BurntSushi.
//!
//! In a nutshell, you give a set of strings to `fst`, and it builds a
//! finite state machine describing this set of strings. The strings which
//! could fuzzy-match a pattern can also be described by a finite state machine.
//! What is freaking cool is that you can now traverse both state machines in
//! lock-step to enumerate the strings which are both in the input set and
//! fuzz-match the query. Or, more formally, given two languages described by
//! FSTs, one can build a product FST which describes the intersection of the
//! languages.
//!
//! `fst` does not support cheap updating of the index, but it supports unioning
//! of state machines. So, to account for changing source code, we build an FST
//! for each library (which is assumed to never change) and an FST for each Rust
//! file in the current workspace, and run a query against the union of all
//! those FSTs.

mod collector;
pub mod sym_db;

use crate::RootDatabase;
use crate::symbol_index::sym_db::{FileSymbol, SymbolIndex};
use base_db::SourceDatabase;
use fst::raw::IndexedValue;
use fst::{Automaton, Streamer};
use std::ops::ControlFlow;
use std::sync::Arc;

// Feature: Workspace Symbol
//
// Uses fuzzy-search to find types, modules and functions by name across your
// project and dependencies. This is **the** most useful feature, which improves code
// navigation tremendously.
pub fn world_symbols(db: &RootDatabase, query: Query) -> Vec<FileSymbol> {
    let _p = tracing::info_span!("world_symbols", query = ?query.query).entered();

    let package_indices = db
        .all_package_ids()
        .data(db)
        .iter()
        .map(|package_id| sym_db::world_symbols_in_package(db, *package_id))
        .collect::<Vec<_>>();

    let mut res = vec![];
    query.search::<()>(&package_indices, |f| {
        res.push(f.clone());
        ControlFlow::Continue(())
    });
    res
}

#[derive(Debug)]
pub struct Query {
    query: String,
    lowercased: String,
    mode: SearchMode,
    case_sensitive: bool,
    // only_types: bool,
    // libs: bool,
}

impl Query {
    pub fn new(query: String) -> Query {
        let lowercased = query.to_lowercase();
        Query {
            query,
            lowercased,
            // only_types: false,
            // libs: false,
            mode: SearchMode::Fuzzy,
            case_sensitive: false,
        }
    }

    // pub fn only_types(&mut self) {
    //     self.only_types = true;
    // }

    // pub fn libs(&mut self) {
    //     self.libs = true;
    // }

    pub fn fuzzy(&mut self) {
        self.mode = SearchMode::Fuzzy;
    }

    pub fn exact(&mut self) {
        self.mode = SearchMode::Exact;
    }

    pub fn prefix(&mut self) {
        self.mode = SearchMode::Prefix;
    }

    pub fn case_sensitive(mut self) -> Self {
        self.case_sensitive = true;
        self
    }
}

impl Query {
    pub(crate) fn search<T>(
        self,
        indices: &[Arc<SymbolIndex>],
        cb: impl FnMut(&FileSymbol) -> ControlFlow<T>,
    ) -> Option<T> {
        let _p = tracing::info_span!("symbol_index::Query::search").entered();
        let mut op = fst::map::OpBuilder::new();
        match self.mode {
            SearchMode::Exact => {
                let automaton = fst::automaton::Str::new(&self.lowercased);

                for index in indices.iter() {
                    op = op.add(index.map.search(&automaton));
                }
                self.search_maps(indices, op.union(), cb)
            }
            SearchMode::Fuzzy => {
                let automaton = fst::automaton::Subsequence::new(&self.lowercased);

                for index in indices.iter() {
                    op = op.add(index.map.search(&automaton));
                }
                self.search_maps(indices, op.union(), cb)
            }
            SearchMode::Prefix => {
                let automaton = fst::automaton::Str::new(&self.lowercased).starts_with();

                for index in indices.iter() {
                    op = op.add(index.map.search(&automaton));
                }
                self.search_maps(indices, op.union(), cb)
            }
        }
    }

    fn search_maps<'sym, T>(
        &self,
        indices: &'sym [Arc<SymbolIndex>],
        mut stream: fst::map::Union<'_>,
        mut cb: impl FnMut(&'sym FileSymbol) -> ControlFlow<T>,
    ) -> Option<T> {
        let ignore_underscore_prefixed = !self.query.starts_with("__");
        while let Some((_, indexed_values)) = stream.next() {
            for &IndexedValue { index, value } in indexed_values {
                let symbol_index = &indices[index];
                let (start, end) = SymbolIndex::map_value_to_range(value);

                for symbol in &symbol_index.symbols[start..end] {
                    // Hide symbols that start with `__` unless the query starts with `__`
                    let symbol_name = symbol.name.as_str();
                    if ignore_underscore_prefixed && symbol_name.starts_with("__") {
                        continue;
                    }
                    if self.mode.check(&self.query, self.case_sensitive, symbol_name) {
                        if let Some(b) = cb(symbol).break_value() {
                            return Some(b);
                        }
                    }
                }
            }
        }
        None
    }
}

/// A way to match import map contents against the search query.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SearchMode {
    /// Import map entry should strictly match the query string.
    Exact,
    /// Import map entry should contain all letters from the query string,
    /// in the same order, but not necessary adjacent.
    Fuzzy,
    /// Import map entry should match the query string by prefix.
    Prefix,
}

impl SearchMode {
    pub fn check(self, query: &str, case_sensitive: bool, candidate: &str) -> bool {
        match self {
            SearchMode::Exact if case_sensitive => candidate == query,
            SearchMode::Exact => candidate.eq_ignore_ascii_case(query),
            SearchMode::Prefix => {
                query.len() <= candidate.len() && {
                    let prefix = &candidate[..query.len()];
                    if case_sensitive {
                        prefix == query
                    } else {
                        prefix.eq_ignore_ascii_case(query)
                    }
                }
            }
            SearchMode::Fuzzy => {
                let mut name = candidate;
                query.chars().all(|query_char| {
                    let m = if case_sensitive {
                        name.match_indices(query_char).next()
                    } else {
                        name.match_indices([query_char, query_char.to_ascii_uppercase()])
                            .next()
                    };
                    match m {
                        Some((index, _)) => {
                            name = name[index..].strip_prefix(|_: char| true).unwrap_or_default();
                            true
                        }
                        None => false,
                    }
                })
            }
        }
    }
}
