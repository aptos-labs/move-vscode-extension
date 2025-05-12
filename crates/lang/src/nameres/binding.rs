use crate::nameres::name_resolution::get_entries_from_walking_scopes;
use crate::nameres::namespaces::{ENUM_VARIANTS, TYPES_N_ENUMS_N_ENUM_VARIANTS_N_MODULES};
use crate::nameres::scope::{ScopeEntry, ScopeEntryListExt, VecExt};
use crate::nameres::{ResolveReference, get_named_field_entries};
use crate::types::ty::Ty;
use base_db::SourceDatabase;
use syntax::SyntaxKind::*;
use syntax::ast::FieldsOwner;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, ast};

pub fn resolve_ident_pat_with_expected_type(
    db: &dyn SourceDatabase,
    ident_pat: InFile<ast::IdentPat>,
    expected_type: Option<Ty>,
) -> Option<ScopeEntry> {
    let binding_entries = get_ident_pat_resolve_variants(db, ident_pat.clone(), false);
    binding_entries
        .filter_by_expected_type(db, expected_type)
        .filter_by_name(ident_pat.value.to_string())
        .single_or_none()
}

pub fn get_ident_pat_resolve_variants(
    db: &dyn SourceDatabase,
    ident_pat: InFile<ast::IdentPat>,
    is_completion: bool,
) -> Vec<ScopeEntry> {
    let (file_id, ident_pat) = ident_pat.unpack();

    let mut entries = vec![];
    if let Some(struct_pat_field) = ident_pat.syntax().parent_of_type::<ast::StructPatField>() {
        let struct_pat = struct_pat_field.struct_pat();
        let fields_owner = struct_pat
            .path()
            .in_file(file_id)
            .resolve_no_inf(db)
            .and_then(|it| it.cast_into::<ast::AnyFieldsOwner>(db));
        // can be null if unresolved
        if let Some(fields_owner) = fields_owner {
            entries.extend(get_named_field_entries(fields_owner));
            if is_completion {
                return entries;
            }
        }
    }

    let ns = if is_completion {
        TYPES_N_ENUMS_N_ENUM_VARIANTS_N_MODULES
    } else {
        ENUM_VARIANTS
    };

    let binding_entries = get_entries_from_walking_scopes(db, ident_pat.in_file(file_id), ns);
    for binding_entry in binding_entries {
        if let Some(named_item) = binding_entry.clone().cast_into::<ast::AnyNamedElement>(db) {
            let is_constant_like = is_constant_like(&named_item);
            let is_path_or_destructuble = matches!(named_item.kind(), ENUM | VARIANT | STRUCT);
            if is_constant_like || (is_completion && is_path_or_destructuble) {
                entries.push(binding_entry);
            }
        };
    }

    entries
}

/**
 * It is possible to match value with constant-like element, e.g.
 *      ```
 *      enum Kind { A }
 *      use Kind::A;
 *      match kind { A => ... } // `A` is a constant-like element, not a pat binding
 *      ```
 *
 * But there is no way to distinguish a pat binding from a constant-like element on syntax level,
 * so we resolve an item `A` first, and then use [isConstantLike] to check whether the element is constant-like or not.
 *
 * Constant-like element can be: real constant, static variable, and enum variant without fields.
 */
fn is_constant_like(named_item: &InFile<ast::AnyNamedElement>) -> bool {
    if named_item.kind() == CONST {
        return true;
    }
    if let Some(fields_owner) = named_item.cast_into_ref::<ast::AnyFieldsOwner>() {
        if fields_owner.value.is_fieldless() {
            return true;
        }
    }
    false
}
