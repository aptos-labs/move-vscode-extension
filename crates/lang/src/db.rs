use crate::loc::{SyntaxLoc, SyntaxLocExt};
use crate::nameres::name_resolution::get_struct_pat_field_resolve_variants;
use crate::nameres::paths;
use crate::nameres::scope::ScopeEntry;
use crate::InFile;
use base_db::{SourceRootDatabase, Upcast};
use parser::SyntaxKind::{PATH, STRUCT_PAT_FIELD};
use stdx::itertools::Itertools;
use syntax::ast::HasReference;
use syntax::{ast, unwrap_or_return};

#[ra_salsa::query_group(HirDatabaseStorage)]
pub trait HirDatabase: SourceRootDatabase + Upcast<dyn SourceRootDatabase> {
    fn resolve_ref_loc(&self, ref_loc: SyntaxLoc) -> Vec<ScopeEntry>;

    #[ra_salsa::transparent]
    fn resolve_ref_multi(&self, any_ref: InFile<ast::AnyHasReference>) -> Vec<ScopeEntry>;

    #[ra_salsa::transparent]
    fn resolve_ref_single(&self, any_ref: InFile<ast::AnyHasReference>) -> Option<ScopeEntry>;
}

fn resolve_ref_loc(db: &dyn HirDatabase, ref_loc: SyntaxLoc) -> Vec<ScopeEntry> {
    match ref_loc.kind() {
        PATH => {
            let path = unwrap_or_return!(ref_loc.cast::<ast::Path>(db.upcast()), vec![]);
            paths::resolve(db, path)
        }
        STRUCT_PAT_FIELD => {
            let struct_pat_field = unwrap_or_return!(ref_loc.cast::<ast::StructPatField>(db.upcast()), vec![]);

            let field_entries = get_struct_pat_field_resolve_variants(db, struct_pat_field);
            tracing::debug!(?field_entries);

            field_entries
        }
        _ => vec![],
    }
}

fn resolve_ref_multi(db: &dyn HirDatabase, any_ref: InFile<ast::AnyHasReference>) -> Vec<ScopeEntry> {
    let path_loc = any_ref.loc();
    db.resolve_ref_loc(path_loc)
}

fn resolve_ref_single(db: &dyn HirDatabase, any_ref: InFile<ast::AnyHasReference>) -> Option<ScopeEntry> {
    let entries = db.resolve_ref_multi(any_ref);
    entries.into_iter().exactly_one().ok()
}
