use crate::db::HirDatabase;
use crate::loc::SyntaxLocFileExt;
use crate::nameres::scope::{NamedItemsExt, NamedItemsInFileExt, ScopeEntry, ScopeEntryListExt, VecExt};
use crate::node_ext::item::ModuleItemExt;
use syntax::ast;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxNodeExt;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::ast::{FieldsOwner, ReferenceElement};
use syntax::files::{InFile, InFileExt};

pub mod address;
pub mod binding;
mod blocks;
pub mod fq_named_element;
mod is_visible;
pub mod name_resolution;
pub mod namespaces;
mod node_ext;
pub mod path_kind;
pub mod path_resolution;
pub mod scope;
mod scope_entries_owner;
pub mod use_speck_entries;

pub trait ResolveReference {
    fn resolve(&self, db: &dyn HirDatabase) -> Option<ScopeEntry>;
    fn resolve_no_inf(&self, db: &dyn HirDatabase) -> Option<ScopeEntry>;
}

impl<T: ast::ReferenceElement> ResolveReference for InFile<T> {
    fn resolve(&self, db: &dyn HirDatabase) -> Option<ScopeEntry> {
        use syntax::SyntaxKind::*;

        let InFile { file_id, value: ref_element } = self;

        let opt_inference_ctx_owner = ref_element
            .syntax()
            .ancestor_or_self::<ast::Expr>()
            .and_then(|expr| expr.inference_ctx_owner().map(|it| it.in_file(*file_id)));
        let msl = self.value.syntax().is_msl_context();
        if let Some(inference_ctx_owner) = opt_inference_ctx_owner {
            let inference = db.inference_for_ctx_owner(inference_ctx_owner.loc(), msl);

            if let Some(method_or_path) = ref_element.cast_into::<ast::MethodOrPath>() {
                let entry = inference.get_resolve_method_or_path(method_or_path.clone());
                if entry.is_none() {
                    // to support qualifier paths, as they're not cached
                    return method_or_path.cast_into::<ast::Path>().and_then(|path| {
                        path_resolution::resolve_path(db, path.in_file(self.file_id), None)
                            .single_or_none()
                    });
                }
                return entry;
            }

            let kind = ref_element.syntax().kind();
            let entry = match kind {
                STRUCT_PAT_FIELD => {
                    let struct_pat_field = ref_element.cast_into::<ast::StructPatField>().unwrap();

                    let struct_path = struct_pat_field.struct_pat().path();
                    let fields_owner = inference
                        .get_resolve_method_or_path(struct_path.into())?
                        .cast_into::<ast::AnyFieldsOwner>(db)?;

                    let field_name = struct_pat_field.field_name()?;
                    get_named_field_entries(fields_owner)
                        .filter_by_name(field_name)
                        .single_or_none()
                }
                STRUCT_LIT_FIELD => {
                    let struct_lit_field = ref_element.cast_into::<ast::StructLitField>().unwrap();

                    let struct_path = struct_lit_field.struct_lit().path();
                    let fields_owner = inference
                        .get_resolve_method_or_path(struct_path.into())?
                        .cast_into::<ast::AnyFieldsOwner>(db)?;

                    let field_name = struct_lit_field.field_name()?;
                    get_named_field_entries(fields_owner)
                        .filter_by_name(field_name)
                        .single_or_none()
                }
                FIELD_REF => {
                    let field_ref = ref_element.cast_into::<ast::FieldRef>().unwrap();
                    inference.get_resolved_field(&field_ref)
                }
                IDENT_PAT => {
                    let ident_pat = ref_element.cast_into::<ast::IdentPat>().unwrap();
                    inference.get_resolved_ident_pat(&ident_pat)
                }
                _ => None,
            };

            return entry;
        }

        // outside inference context
        self.resolve_no_inf(db)
    }

    fn resolve_no_inf(&self, db: &dyn HirDatabase) -> Option<ScopeEntry> {
        // outside inference context
        if let Some(item_spec_ref) = self.cast_into_ref::<ast::ItemSpecRef>() {
            let ref_name = item_spec_ref.value.name_ref()?.as_string();
            let item_spec = item_spec_ref.map(|it| it.item_spec());
            let module = item_spec.module(db)?;
            let verified_items = module.map(|it| it.verifiable_items()).flatten().to_entries();
            return verified_items.filter_by_name(ref_name).single_or_none();
        }

        let path = self.cast_into_ref::<ast::Path>()?;
        db.resolve_path(path.loc())
    }
}

fn get_named_field_entries(fields_owner: InFile<ast::AnyFieldsOwner>) -> Vec<ScopeEntry> {
    let InFile { file_id, value: fields_owner } = fields_owner;
    fields_owner.named_fields().to_in_file_entries(file_id)
}
