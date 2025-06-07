#![cfg(test)]

mod fold;
mod hover;
mod ide_test_utils;
mod resolve;
mod test_completion;
mod test_inlay_hints;
mod test_replace_with_compound_expr;
mod test_replace_with_method_call;
mod test_syntax_highlighting;
mod test_unresolved_reference;
mod types;

mod test_completion_functions;
mod test_completion_loops;
mod test_completion_methods;
mod test_find_usages;
mod test_load_dependencies;
mod test_resolve_items;
mod test_resolve_types;
mod test_type_checking;
mod test_view_syntax_tree;

pub use test_utils::tracing::init_tracing_for_test;
