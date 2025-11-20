// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

#![allow(unused)]

mod fold;
pub mod ide_test_utils;
mod resolve;
mod test_completion;
mod test_inlay_type_hints;
mod test_quick_docs;
mod test_syntax_highlighting;
mod types;

mod test_abilities_key;
mod test_code_lens;
mod test_completion_functions;
mod test_completion_loops;
mod test_completion_methods;
mod test_completion_out_of_scope;
mod test_completion_relevance;
mod test_diagnostics;
mod test_error_const_docs;
mod test_find_usages;
mod test_goto_specification;
mod test_inlay_hints;
mod test_inlay_parameter_hints;
mod test_load_dependencies;
mod test_named_addresses;
mod test_organize_imports;
mod test_rename;
mod test_resolve_items;
mod test_resolve_types;
mod test_signature_help_struct_lit_fields;
mod test_signature_help_type_parameters;
mod test_signature_help_value_parameters;
mod test_view_syntax_tree;
mod test_world_symbols;

pub use test_utils::tracing::init_tracing_for_test;
