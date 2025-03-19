use crate::loc::{SyntaxLoc, SyntaxLocExt};
use crate::nameres::name_resolution::{
    get_struct_lit_field_resolve_variants, get_struct_pat_field_resolve_variants,
};
use crate::nameres::paths;
use crate::nameres::scope::{ScopeEntry, ScopeEntryListExt};
use crate::node_ext::struct_field_name::StructFieldNameExt;
use crate::types::inference::inference_result::InferenceResult;
use crate::types::inference::InferenceCtx;
use crate::{AsName, InFile};
use base_db::{SourceRootDatabase, Upcast};
use parser::SyntaxKind::{PATH, STRUCT_LIT_FIELD, STRUCT_PAT_FIELD};
use stdx::itertools::Itertools;
use syntax::ast::HasName;
use syntax::{ast, unwrap_or_return};

#[ra_salsa::query_group(HirDatabaseStorage)]
pub trait HirDatabase: SourceRootDatabase + Upcast<dyn SourceRootDatabase> {
    fn resolve_ref_loc(&self, ref_loc: SyntaxLoc) -> Vec<ScopeEntry>;

    #[ra_salsa::transparent]
    fn resolve_ref_multi(&self, any_ref: InFile<ast::AnyHasReference>) -> Vec<ScopeEntry>;

    #[ra_salsa::transparent]
    fn resolve_ref_single(&self, any_ref: InFile<ast::AnyHasReference>) -> Option<ScopeEntry>;

    fn inference(&self, ctx_owner_loc: SyntaxLoc) -> Option<InferenceResult>;

    #[ra_salsa::transparent]
    fn inference_for_ctx_owner(
        &self,
        ctx_owner: InFile<ast::InferenceCtxOwner>,
    ) -> Option<InferenceResult>;
}

fn resolve_ref_loc(db: &dyn HirDatabase, ref_loc: SyntaxLoc) -> Vec<ScopeEntry> {
    match ref_loc.kind() {
        STRUCT_PAT_FIELD => {
            let struct_pat_field = ref_loc.cast::<ast::StructPatField>(db.upcast()).unwrap();
            let Some(struct_pat_field_name) = struct_pat_field.value.field_name() else {
                return vec![];
            };
            let field_entries = get_struct_pat_field_resolve_variants(db, struct_pat_field);
            tracing::debug!(?struct_pat_field_name, ?field_entries);

            field_entries
                .into_iter()
                .filter_by_name(struct_pat_field_name)
                .collect()
        }
        STRUCT_LIT_FIELD => {
            let struct_lit_field = ref_loc.cast::<ast::StructLitField>(db.upcast()).unwrap();
            let Some(struct_lit_field_name) = struct_lit_field.value.field_name() else {
                return vec![];
            };
            let field_entries = get_struct_lit_field_resolve_variants(db, struct_lit_field);
            tracing::debug!(?struct_lit_field_name, ?field_entries);

            field_entries
                .into_iter()
                .filter_by_name(struct_lit_field_name)
                .collect()
        }
        PATH => {
            let path = unwrap_or_return!(ref_loc.cast::<ast::Path>(db.upcast()), vec![]);
            paths::resolve(db, path)
        }
        _ => vec![],
    }
}

fn resolve_ref_multi(db: &dyn HirDatabase, any_ref: InFile<ast::AnyHasReference>) -> Vec<ScopeEntry> {
    db.resolve_ref_loc(any_ref.loc())
}

fn resolve_ref_single(
    db: &dyn HirDatabase,
    any_ref: InFile<ast::AnyHasReference>,
) -> Option<ScopeEntry> {
    let entries = db.resolve_ref_multi(any_ref);
    entries.into_iter().exactly_one().ok()
}

fn inference(db: &dyn HirDatabase, ctx_owner_loc: SyntaxLoc) -> Option<InferenceResult> {
    let Some(ctx_owner) = ctx_owner_loc.cast::<ast::InferenceCtxOwner>(db.upcast()) else {
        return None;
    };
    let ctx = InferenceCtx::new(db, ctx_owner.file_id);

    let inference_result = ctx.infer(ctx_owner);
    Some(inference_result)
}

fn inference_for_ctx_owner(
    db: &dyn HirDatabase,
    ctx_owner: InFile<ast::InferenceCtxOwner>,
) -> Option<InferenceResult> {
    let ctx_owner_loc = ctx_owner.loc();
    db.inference(ctx_owner_loc)
}
