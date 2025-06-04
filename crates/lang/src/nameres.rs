use crate::hir_db;
use crate::nameres::labels::get_loop_labels_resolve_variants;
use crate::nameres::path_resolution::remove_variant_ident_pats;
use crate::nameres::scope::{NamedItemsExt, ScopeEntry, ScopeEntryListExt, VecExt};
use crate::node_ext::item::ModuleItemExt;
use crate::node_ext::item_spec::ItemSpecExt;
use base_db::SourceDatabase;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::ast::{FieldsOwner, GenericElement};
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, ast};

pub mod address;
pub mod binding;
mod blocks;
pub mod fq_named_element;
mod is_visible;
mod labels;
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
    resolve_multi(db, ref_element)?.single_or_none()
}

pub fn resolve_multi(
    db: &dyn SourceDatabase,
    ref_element: InFile<impl Into<ast::ReferenceElement>>,
) -> Option<Vec<ScopeEntry>> {
    let (file_id, ref_element) = ref_element.map(|it| it.into()).unpack();

    match ref_element.clone() {
        ast::ReferenceElement::ItemSpecRef(item_spec_ref) => {
            return get_item_spec_entries(db, item_spec_ref.in_file(file_id));
        }
        ast::ReferenceElement::Label(loop_label) => {
            let label = loop_label.in_file(file_id);
            let label_name = label.value.name_as_string();
            let loop_label_entries = get_loop_labels_resolve_variants(label).filter_by_name(label_name);
            tracing::debug!(?loop_label_entries);
            return Some(loop_label_entries);
        }
        ast::ReferenceElement::ItemSpecTypeParam(item_spec_type_param) => {
            let item_spec_fun = item_spec_type_param
                .item_spec()
                .in_file(file_id)
                .item(db)?
                .cast_into::<ast::Fun>()?;
            let entries = item_spec_fun.flat_map(|it| it.type_params()).to_entries();
            return Some(entries);
        }
        _ => (),
    }

    // skip path in AttrItem = Path '=' Expr
    if let ast::ReferenceElement::Path(path) = &ref_element {
        if path.root_parent_of_type::<ast::AttrItem>().is_some() {
            return None;
        }
    }

    let ctx_owner = ref_element.syntax().inference_ctx_owner();
    let msl = ref_element.syntax().is_msl_context();

    if let Some(ctx_owner) = ctx_owner {
        let inference = hir_db::inference(db, ctx_owner.in_file(file_id), msl);

        let entries = match ref_element {
            ast::ReferenceElement::MethodCallExpr(method_call) => {
                let entries = inference.get_resolve_method_or_path_entries(method_call.into());
                entries
            }
            ast::ReferenceElement::Path(path) => {
                let entries = inference.get_resolve_method_or_path_entries(path.clone().into());
                if entries.is_empty() {
                    let _p = tracing::debug_span!("fallback_resolve_multi_for_path").entered();
                    // to support qualifier paths, as they're not cached
                    let entries = path_resolution::resolve_path(db, path.in_file(file_id), None);
                    let filtered_entries =
                        remove_variant_ident_pats(db, entries, |ident_pat| resolve(db, ident_pat));
                    return Some(filtered_entries);
                }
                entries
            }
            ast::ReferenceElement::StructPatField(struct_pat_field) => {
                let struct_path = struct_pat_field.struct_pat().path();
                let fields_owner = inference
                    .get_resolve_method_or_path(struct_path.into())?
                    .cast_into::<ast::AnyFieldsOwner>(db)?;

                let field_name = struct_pat_field.field_name()?;
                get_named_field_entries(fields_owner).filter_by_name(field_name)
            }
            ast::ReferenceElement::StructLitField(struct_lit_field) => {
                let struct_path = struct_lit_field.struct_lit().path();
                let fields_owner = inference
                    .get_resolve_method_or_path(struct_path.into())?
                    .cast_into::<ast::AnyFieldsOwner>(db)?;

                let field_name = struct_lit_field.field_name()?;
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
            _ => vec![],
        };
        return Some(entries);
    }

    // outside inference context
    resolve_no_inf_multi(db, ref_element.in_file(file_id))
}

pub fn resolve_no_inf_cast<Named: ast::NamedElement>(
    db: &dyn SourceDatabase,
    ref_element: InFile<impl Into<ast::ReferenceElement>>,
) -> Option<InFile<Named>> {
    resolve_no_inf_multi(db, ref_element)?
        .single_or_none()?
        .cast_into::<Named>(db)
}

pub fn resolve_no_inf(
    db: &dyn SourceDatabase,
    ref_element: InFile<impl Into<ast::ReferenceElement>>,
) -> Option<ScopeEntry> {
    resolve_no_inf_multi(db, ref_element)?.single_or_none()
}

/// resolve outside of the `ast::InferenceCtxOwner`
fn resolve_no_inf_multi(
    db: &dyn SourceDatabase,
    ref_element: InFile<impl Into<ast::ReferenceElement>>,
) -> Option<Vec<ScopeEntry>> {
    let ref_element = ref_element.map(|it| it.into());
    match ref_element.cast_into_ref::<ast::Path>() {
        Some(path) => Some(hir_db::resolve_path_multi(db, path)),
        None => {
            tracing::debug!("cannot resolve {:?} without inference", ref_element.kind());
            None
        }
    }
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

fn get_named_field_entries(fields_owner: InFile<ast::AnyFieldsOwner>) -> Vec<ScopeEntry> {
    fields_owner.flat_map(|it| it.named_fields()).to_entries()
}

fn get_schema_field_entries(schema: InFile<ast::Schema>) -> Vec<ScopeEntry> {
    schema.flat_map(|it| it.schema_fields()).to_entries()
}
