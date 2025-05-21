use crate::hir_db;
use crate::nameres::labels::get_loop_labels_resolve_variants;
use crate::nameres::path_resolution::remove_variant_ident_pats;
use crate::nameres::scope::{NamedItemsExt, ScopeEntry, ScopeEntryListExt, VecExt};
use crate::node_ext::item::ModuleItemExt;
use base_db::SourceDatabase;
use syntax::ast;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::ast::{FieldsOwner, ReferenceElement};
use syntax::files::{InFile, InFileExt};

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

pub trait ResolveReference {
    fn resolve_multi(&self, db: &dyn SourceDatabase) -> Option<Vec<ScopeEntry>>;
    fn resolve(&self, db: &dyn SourceDatabase) -> Option<ScopeEntry>;
    fn resolve_no_inf_multi(&self, db: &dyn SourceDatabase) -> Option<Vec<ScopeEntry>>;
    fn resolve_no_inf(&self, db: &dyn SourceDatabase) -> Option<ScopeEntry>;
}

impl<T: ReferenceElement> ResolveReference for InFile<T> {
    fn resolve_multi(&self, db: &dyn SourceDatabase) -> Option<Vec<ScopeEntry>> {
        use syntax::SyntaxKind::*;

        if let Some(item_spec_ref) = self.cast_into_ref::<ast::ItemSpecRef>() {
            return get_item_spec_entries(db, item_spec_ref);
        }

        let InFile { file_id, value: ref_element } = self;

        if let Some(loop_label) = ref_element.cast_into::<ast::Label>() {
            let label = loop_label.in_file(*file_id);
            let label_name = label.value.name_as_string();
            let filtered_entries = get_loop_labels_resolve_variants(label).filter_by_name(label_name);
            tracing::debug!(?filtered_entries);
            return Some(filtered_entries);
        }

        let opt_inference_ctx_owner = ref_element
            .syntax()
            .ancestor_or_self::<ast::InferenceCtxOwner>()
            .map(|it| it.in_file(*file_id));

        let msl = self.value.syntax().is_msl_context();
        if let Some(ctx_owner) = opt_inference_ctx_owner {
            let inference = hir_db::inference(db, ctx_owner, msl);

            if let Some(method_or_path) = ref_element.cast_into::<ast::MethodOrPath>() {
                let entries = inference.get_resolve_method_or_path_entries(method_or_path.clone());
                if entries.is_empty() {
                    // to support qualifier paths, as they're not cached
                    let method_or_path = method_or_path.cast_into::<ast::Path>()?;
                    let entries =
                        path_resolution::resolve_path(db, method_or_path.in_file(self.file_id), None);
                    let filtered_entries = remove_variant_ident_pats(db, entries, |it| it.resolve(db));
                    return Some(filtered_entries);
                }
                return Some(entries);
            }

            let kind = ref_element.syntax().kind();
            let entries = match kind {
                STRUCT_PAT_FIELD => {
                    let struct_pat_field = ref_element.cast_into::<ast::StructPatField>().unwrap();

                    let struct_path = struct_pat_field.struct_pat().path();
                    let fields_owner = inference
                        .get_resolve_method_or_path(struct_path.into())?
                        .cast_into::<ast::AnyFieldsOwner>(db)?;

                    let field_name = struct_pat_field.field_name()?;
                    get_named_field_entries(fields_owner).filter_by_name(field_name)
                    // .single_or_none()
                }
                STRUCT_LIT_FIELD => {
                    let struct_lit_field = ref_element.cast_into::<ast::StructLitField>().unwrap();

                    let struct_path = struct_lit_field.struct_lit().path();
                    let fields_owner = inference
                        .get_resolve_method_or_path(struct_path.into())?
                        .cast_into::<ast::AnyFieldsOwner>(db)?;

                    let field_name = struct_lit_field.field_name()?;
                    get_named_field_entries(fields_owner).filter_by_name(field_name)
                }
                SCHEMA_LIT_FIELD => {
                    let schema_lit_field = ref_element.cast_into::<ast::SchemaLitField>().unwrap();

                    let schema_lit_path = schema_lit_field.schema_lit()?.path()?;
                    let schema = inference
                        .get_resolve_method_or_path(schema_lit_path.into())?
                        .cast_into::<ast::Schema>(db)?;

                    let field_name = schema_lit_field.field_name()?;
                    get_schema_field_entries(schema).filter_by_name(field_name)
                }
                DOT_EXPR => {
                    let dot_expr = ref_element.cast_into::<ast::DotExpr>().unwrap();
                    let field_name_ref = dot_expr.name_ref()?;
                    inference
                        .get_resolved_field(&field_name_ref)
                        .map(|it| vec![it])
                        .unwrap_or_default()
                }
                IDENT_PAT => {
                    let ident_pat = ref_element.cast_into::<ast::IdentPat>().unwrap();
                    inference
                        .get_resolved_ident_pat(&ident_pat)
                        .map(|it| vec![it])
                        .unwrap_or_default()
                }
                _ => return None,
            };
            return Some(entries);
        }

        // outside inference context
        self.resolve_no_inf_multi(db)
    }

    fn resolve(&self, db: &dyn SourceDatabase) -> Option<ScopeEntry> {
        self.resolve_multi(db)?.single_or_none()
    }

    /// resolve outside of the `ast::InferenceCtxOwner`
    fn resolve_no_inf_multi(&self, db: &dyn SourceDatabase) -> Option<Vec<ScopeEntry>> {
        match self.cast_into_ref::<ast::Path>() {
            Some(path) => Some(hir_db::resolve_path_multi(db, path)),
            None => {
                let kind = self.value.syntax().kind();
                tracing::debug!("cannot resolve {:?} without inference", kind);
                None
            }
        }
    }

    fn resolve_no_inf(&self, db: &dyn SourceDatabase) -> Option<ScopeEntry> {
        self.resolve_no_inf_multi(db)?.single_or_none()
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
