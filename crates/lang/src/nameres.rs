use crate::db::HirDatabase;
use crate::files::InFileExt;
use crate::nameres::scope::{NamedItemsInFileExt, ScopeEntry, ScopeEntryListExt, VecExt};
use crate::node_ext::struct_field_name::StructFieldNameExt;
use crate::InFile;
use syntax::ast;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::ast::{FieldsOwner, ReferenceElement};

pub mod address;
mod blocks;
mod is_visible;
pub mod name_resolution;
pub mod namespaces;
mod node_ext;
pub mod path_kind;
pub mod path_resolution;
pub mod scope;
mod scope_entries_owner;
pub mod use_speck_entries;

impl<T: ast::ReferenceElement> InFile<T> {
    pub fn resolve(&self, db: &dyn HirDatabase) -> Option<ScopeEntry> {
        use syntax::SyntaxKind::*;

        let InFile {
            file_id,
            value: ref_element,
        } = self;

        let opt_inference_ctx_owner = ref_element
            .syntax()
            .ancestor_or_self::<ast::Expr>()
            .and_then(|expr| expr.inference_ctx_owner().map(|it| it.in_file(*file_id)));

        if let Some(inference_ctx_owner) = opt_inference_ctx_owner {
            let inference = inference_ctx_owner.inference(db);

            if let Some(method_or_path) = ref_element.cast_into::<ast::MethodOrPath>() {
                let entry = inference.resolve_method_or_path(method_or_path.clone());
                if entry.is_none() {
                    // temporary fallback till full infer implementation is ready
                    return method_or_path.cast_into::<ast::Path>().and_then(|path| {
                        path_resolution::resolve_path(db, path.in_file(self.file_id)).single_or_none()
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
                        .resolve_method_or_path(struct_path.into())?
                        .cast_into::<ast::AnyFieldsOwner>(db)?;

                    let field_name = struct_pat_field.field_name()?;
                    get_named_field_entries(fields_owner)
                        .filter_by_name(field_name)
                        .single_or_none()
                }
                STRUCT_LIT_FIELD => {
                    let struct_lit_fields = ref_element.cast_into::<ast::StructLitField>().unwrap();

                    let struct_path = struct_lit_fields.struct_lit().path();
                    let fields_owner = inference
                        .resolve_method_or_path(struct_path.into())?
                        .cast_into::<ast::AnyFieldsOwner>(db)?;

                    let field_name = struct_lit_fields.field_name()?;
                    get_named_field_entries(fields_owner)
                        .filter_by_name(field_name)
                        .single_or_none()
                }
                FIELD_REF => {
                    let field_ref = ref_element.cast_into::<ast::FieldRef>().unwrap();
                    inference.get_resolved_field(&field_ref)
                }
                _ => None,
            };

            return entry;
        }

        // outside inference context
        ref_element.cast_into::<ast::Path>().and_then(|path| {
            path_resolution::resolve_path(db, path.in_file(self.file_id)).single_or_none()
        })
    }

    pub fn resolve_no_inf(&self, db: &dyn HirDatabase) -> Option<ScopeEntry> {
        use syntax::SyntaxKind::*;

        let InFile {
            file_id: _,
            value: ref_element,
        } = self;

        // outside inference context
        ref_element.cast_into::<ast::Path>().and_then(|path| {
            path_resolution::resolve_path(db, path.in_file(self.file_id)).single_or_none()
        })
    }
}

fn get_named_field_entries(fields_owner: InFile<ast::AnyFieldsOwner>) -> Vec<ScopeEntry> {
    let InFile {
        file_id,
        value: fields_owner,
    } = fields_owner;
    fields_owner.named_fields().to_in_file_entries(file_id)
}
