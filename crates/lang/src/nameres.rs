// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::hir_db;
use crate::nameres::labels::get_loop_labels_resolve_variants;
use crate::nameres::path_resolution::remove_variant_ident_pats;
use crate::nameres::scope::{NamedItemsExt, ScopeEntry, ScopeEntryListExt, VecExt};
use crate::node_ext::item::ModuleItemExt;
use crate::node_ext::item_spec::ItemSpecExt;
use crate::types::inference::inference_result::InferenceResult;
use base_db::SourceDatabase;
use std::sync::Arc;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, ast};

pub mod address;
pub mod binding;
mod blocks;
pub mod fq_named_element;
mod is_visible;
pub mod labels;
pub mod name_resolution;
pub mod namespaces;
pub mod node_ext;
pub mod path_kind;
pub mod path_resolution;
pub mod scope;
mod scope_entries_owner;
pub mod use_speck_entries;

pub fn resolve(
    db: &dyn SourceDatabase,
    ref_element: InFile<impl Into<ast::ReferenceElement>>,
) -> Option<ScopeEntry> {
    resolve_multi(db, ref_element, None)?.single_or_none()
}

pub fn resolve_multi(
    db: &dyn SourceDatabase,
    ref_element: InFile<impl Into<ast::ReferenceElement>>,
    cached_inference: Option<Arc<InferenceResult>>,
) -> Option<Vec<ScopeEntry>> {
    let ref_element = ref_element.map(|it| it.into());
    {
        let (file_id, ref_element) = ref_element.clone().unpack();
        match ref_element.clone() {
            ast::ReferenceElement::ItemSpecRef(item_spec_ref) => {
                return get_item_spec_entries(db, item_spec_ref.in_file(file_id));
            }
            ast::ReferenceElement::Label(loop_label) => {
                let label = loop_label.in_file(file_id);
                let label_name = label.value.name_as_string();
                let loop_label_entries =
                    get_loop_labels_resolve_variants(label).filter_by_name(label_name);
                return Some(loop_label_entries);
            }
            ast::ReferenceElement::ItemSpecTypeParam(item_spec_type_param) => {
                let item_spec_fun = item_spec_type_param
                    .item_spec()
                    .in_file(file_id)
                    .item(db)?
                    .cast_into::<ast::Fun>()?;
                let entries = item_spec_fun
                    .flat_map(|it| it.to_any_fun().to_generic_element().type_params())
                    .to_entries();
                return Some(entries);
            }
            _ => (),
        }
    }

    // skip path in AttrItem = Path '=' Expr
    if let ast::ReferenceElement::Path(path) = &ref_element.value {
        if path.root_parent_of_type::<ast::AttrItem>().is_some() {
            return None;
        }
    }

    let mut inference = cached_inference;
    if inference.is_none() {
        let msl = ref_element.value.syntax().is_msl_context();
        inference = ref_element
            .syntax()
            .and_then(|it| it.inference_ctx_owner())
            .map(|ctx_owner| hir_db::inference(db, ctx_owner, msl));
    }

    match inference {
        Some(inference) => resolve_multi_with_inf(db, inference, ref_element),
        None => resolve_multi_no_inf(db, ref_element),
    }
}

pub fn resolve_no_inf_cast<N: Into<ast::NamedElement> + AstNode>(
    db: &dyn SourceDatabase,
    ref_element: InFile<impl Into<ast::ReferenceElement>>,
) -> Option<InFile<N>> {
    resolve_multi_no_inf(db, ref_element)?
        .single_or_none()?
        .cast_into(db)
}

pub fn resolve_no_inf(
    db: &dyn SourceDatabase,
    ref_element: InFile<impl Into<ast::ReferenceElement>>,
) -> Option<ScopeEntry> {
    resolve_multi_no_inf(db, ref_element)?.single_or_none()
}

/// resolve outside of the `ast::InferenceCtxOwner`
fn resolve_multi_no_inf(
    db: &dyn SourceDatabase,
    ref_element: InFile<impl Into<ast::ReferenceElement>>,
) -> Option<Vec<ScopeEntry>> {
    let (file_id, ref_element) = ref_element.map(|it| it.into()).unpack();
    match ref_element {
        ast::ReferenceElement::Path(path) => Some(hir_db::resolve_path_multi(db, path.in_file(file_id))),
        ast::ReferenceElement::StructLitField(struct_lit_field) => {
            let struct_path = struct_lit_field.struct_lit().path();
            let fields_owner =
                resolve_no_inf_cast::<ast::FieldsOwner>(db, struct_path.in_file(file_id))?;
            let field_name = struct_lit_field.field_name()?.as_string();
            Some(get_named_field_entries(fields_owner).filter_by_name(field_name))
        }
        _ => {
            tracing::debug!(
                "cannot resolve {:?} without inference",
                ref_element.syntax().kind()
            );
            None
        }
    }
}

fn resolve_multi_with_inf(
    db: &dyn SourceDatabase,
    inference: Arc<InferenceResult>,
    ref_element: InFile<ast::ReferenceElement>,
) -> Option<Vec<ScopeEntry>> {
    let (file_id, ref_element) = ref_element.unpack();
    let entries = match ref_element {
        ast::ReferenceElement::MethodCallExpr(method_call) => {
            let entries = inference.get_resolve_method_or_path_entries(method_call.into());
            entries
        }
        ast::ReferenceElement::Path(path) => {
            let entries = inference.get_resolve_method_or_path_entries(path.clone().into());
            if entries.is_empty() {
                // to support qualifier paths, as they're not cached
                return Some(fallback_resolve_multi_for_path(db, path.in_file(file_id)));
            }
            entries
        }
        ast::ReferenceElement::StructPatField(struct_pat_field) => {
            let struct_path = struct_pat_field.struct_pat().path();
            let fields_owner = inference
                .get_resolve_method_or_path(struct_path.into())?
                .cast_into::<ast::FieldsOwner>(db)?;

            let field_name = struct_pat_field.field_name()?;
            get_named_field_entries(fields_owner).filter_by_name(field_name)
        }
        ast::ReferenceElement::StructLitField(struct_lit_field) => {
            let struct_path = struct_lit_field.struct_lit().path();
            let fields_owner = inference
                .get_resolve_method_or_path(struct_path.into())?
                .cast_into::<ast::FieldsOwner>(db)?;

            let field_name = struct_lit_field.field_name()?.as_string();
            get_named_field_entries(fields_owner).filter_by_name(field_name)
        }
        ast::ReferenceElement::SchemaLitField(schema_lit_field) => {
            let schema_lit_path = schema_lit_field.schema_lit()?.path()?;
            let schema = inference
                .get_resolve_method_or_path(schema_lit_path.into())?
                .cast_into::<ast::Schema>(db)?;

            let field_name = schema_lit_field.field_name()?;
            get_schema_field_entries(schema).filter_by_name(field_name)
        }
        ast::ReferenceElement::DotExpr(dot_expr) => {
            let field_name_ref = dot_expr.name_ref()?;
            inference
                .get_resolved_field(&field_name_ref)
                .map(|it| vec![it])
                .unwrap_or_default()
        }
        ast::ReferenceElement::IdentPat(ident_pat) => inference
            .get_resolved_ident_pat(&ident_pat)
            .map(|it| vec![it])
            .unwrap_or_default(),

        // should be unreachable
        _ => vec![],
    };
    Some(entries)
}

#[tracing::instrument(level = "debug", skip_all)]
fn fallback_resolve_multi_for_path(db: &dyn SourceDatabase, path: InFile<ast::Path>) -> Vec<ScopeEntry> {
    let entries = path_resolution::resolve_path(db, path, None);
    let filtered_entries = remove_variant_ident_pats(db, entries, |ident_pat| resolve(db, ident_pat));
    filtered_entries
}

fn get_item_spec_entries(
    db: &dyn SourceDatabase,
    item_spec_ref: InFile<ast::ItemSpecRef>,
) -> Option<Vec<ScopeEntry>> {
    let ref_name = item_spec_ref.value.name_ref()?.as_string();
    let item_spec = item_spec_ref.map(|it| it.item_spec());
    let module = item_spec.module(db)?;
    let verifiable_items = module.map(|it| it.verifiable_items()).flatten().to_entries();
    Some(verifiable_items.filter_by_name(ref_name))
}

fn get_named_field_entries(fields_owner: InFile<ast::FieldsOwner>) -> Vec<ScopeEntry> {
    fields_owner.flat_map(|it| it.named_fields()).to_entries()
}

fn get_schema_field_entries(schema: InFile<ast::Schema>) -> Vec<ScopeEntry> {
    schema.flat_map(|it| it.schema_fields_as_bindings()).to_entries()
}
