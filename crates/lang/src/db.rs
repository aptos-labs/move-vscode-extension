use crate::loc::{SyntaxLoc, SyntaxLocExt};
use crate::nameres::name_resolution::{
    get_struct_lit_field_resolve_variants, get_struct_pat_field_resolve_variants,
};
use crate::nameres::paths;
use crate::nameres::scope::{ScopeEntry, ScopeEntryListExt};
use crate::node_ext::struct_field_name::StructFieldNameExt;
use crate::types::inference::ast_walker::TypeAstWalker;
use crate::types::inference::inference_result::InferenceResult;
use crate::types::inference::InferenceCtx;
use crate::InFile;
use base_db::{SourceRootDatabase, Upcast};
use parser::SyntaxKind::{PATH, STRUCT_LIT_FIELD, STRUCT_PAT_FIELD};
use stdx::itertools::Itertools;
use syntax::ast::NamedElement;
use syntax::{ast, unwrap_or_return};

#[ra_salsa::query_group(HirDatabaseStorage)]
pub trait HirDatabase: SourceRootDatabase + Upcast<dyn SourceRootDatabase> {
    fn resolve_ref_loc(&self, ref_loc: SyntaxLoc) -> Vec<ScopeEntry>;

    #[ra_salsa::transparent]
    fn multi_resolve(&self, any_ref: InFile<ast::AnyReference>) -> Vec<ScopeEntry>;

    #[ra_salsa::transparent]
    fn resolve(&self, any_ref: InFile<ast::AnyReference>) -> Option<ScopeEntry>;

    #[ra_salsa::transparent]
    fn resolve_named_item(
        &self,
        reference: InFile<ast::AnyReference>,
    ) -> Option<InFile<ast::AnyNamedElement>>;

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
            let struct_pat_field = ref_loc.cast_into::<ast::StructPatField>(db.upcast()).unwrap();
            let Some(struct_pat_field_name) = struct_pat_field.value.field_name() else {
                return vec![];
            };
            let field_entries = get_struct_pat_field_resolve_variants(db, struct_pat_field);
            tracing::debug!(?struct_pat_field_name, ?field_entries);

            field_entries.filter_by_name(struct_pat_field_name)
        }
        STRUCT_LIT_FIELD => {
            let struct_lit_field = ref_loc.cast_into::<ast::StructLitField>(db.upcast()).unwrap();
            let Some(struct_lit_field_name) = struct_lit_field.value.field_name() else {
                return vec![];
            };
            let field_entries = get_struct_lit_field_resolve_variants(db, struct_lit_field);
            tracing::debug!(?struct_lit_field_name, ?field_entries);

            field_entries.filter_by_name(struct_lit_field_name)
        }
        PATH => {
            let path = unwrap_or_return!(ref_loc.cast_into::<ast::Path>(db.upcast()), vec![]);
            paths::resolve(db, path)
        }
        _ => vec![],
    }
}

fn multi_resolve(db: &dyn HirDatabase, any_ref: InFile<ast::AnyReference>) -> Vec<ScopeEntry> {
    db.resolve_ref_loc(any_ref.loc())
}

fn resolve(db: &dyn HirDatabase, any_ref: InFile<ast::AnyReference>) -> Option<ScopeEntry> {
    let entries = db.multi_resolve(any_ref);
    entries.into_iter().exactly_one().ok()
}

fn resolve_named_item(
    db: &dyn HirDatabase,
    reference: InFile<ast::AnyReference>,
) -> Option<InFile<ast::AnyNamedElement>> {
    db.resolve(reference)
        .and_then(|it| it.node_loc.cast_into::<ast::AnyNamedElement>(db.upcast()))
}

fn inference(db: &dyn HirDatabase, ctx_owner_loc: SyntaxLoc) -> Option<InferenceResult> {
    let Some(InFile {
        file_id,
        value: ctx_owner,
    }) = ctx_owner_loc.cast_into::<ast::InferenceCtxOwner>(db.upcast())
    else {
        return None;
    };
    let mut ctx = InferenceCtx::new(db, file_id);

    TypeAstWalker::new(&mut ctx).walk(ctx_owner);

    let res = InferenceResult::from_ctx(ctx);
    Some(res)
}

fn inference_for_ctx_owner(
    db: &dyn HirDatabase,
    ctx_owner: InFile<ast::InferenceCtxOwner>,
) -> Option<InferenceResult> {
    let ctx_owner_loc = ctx_owner.loc();
    db.inference(ctx_owner_loc)
}
