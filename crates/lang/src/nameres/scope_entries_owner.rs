use crate::db::HirDatabase;
use crate::nameres::blocks::get_entries_in_blocks;
use crate::nameres::node_ext::ModuleResolutionExt;
use crate::nameres::scope::{NamedItemsExt, NamedItemsInFileExt, ScopeEntry, ScopeEntryExt};
use crate::nameres::use_speck_entries::use_speck_entries;
use base_db::SourceDatabase;
use syntax::ast::{GenericElement, HasItems};
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, SyntaxNode, ast};

pub fn get_entries_in_scope(
    db: &dyn HirDatabase,
    scope: InFile<SyntaxNode>,
    prev: Option<SyntaxNode>,
) -> Vec<ScopeEntry> {
    let mut entries = vec![];
    if let Some(use_stmts_owner) = ast::AnyHasUseStmts::cast(scope.value.clone()) {
        entries.extend(use_speck_entries(
            db,
            &InFile::new(scope.file_id, use_stmts_owner),
        ));
    }

    entries.extend(get_entries_in_blocks(scope.clone(), prev));
    entries.extend(get_entries_from_owner(db, scope));
    entries
}

pub fn get_entries_from_owner(db: &dyn HirDatabase, scope: InFile<SyntaxNode>) -> Vec<ScopeEntry> {
    use syntax::SyntaxKind::*;

    let file_id = scope.file_id;
    let mut entries = vec![];

    if let Some(has_type_params) = ast::AnyGenericElement::cast(scope.value.clone()) {
        entries.extend(has_type_params.type_params().to_in_file_entries(file_id));
    }

    match scope.value.kind() {
        MODULE => {
            let module = scope.syntax_cast::<ast::Module>().unwrap();
            entries.extend(module.member_entries());
            entries.extend(module.value.enum_variants().to_in_file_entries(file_id));

            entries.extend(builtin_functions(db.upcast()).to_entries());
            entries.extend(builtin_spec_functions(db.upcast()).to_entries());
        }
        MODULE_SPEC => {
            let (module_spec_file_id, module_spec) =
                scope.syntax_cast::<ast::ModuleSpec>().unwrap().unpack();
            entries.extend(
                module_spec
                    .spec_functions()
                    .to_in_file_entries(module_spec_file_id),
            );
            entries.extend(
                module_spec
                    .spec_inline_functions()
                    .to_in_file_entries(module_spec_file_id),
            );
            entries.extend(builtin_spec_functions(db.upcast()).to_entries());
        }
        // todo: ITEM_SPEC should have access to params / fields of the item
        SCRIPT => {
            let script = scope.syntax_cast::<ast::Script>().unwrap();
            entries.extend(script.value.consts().to_in_file_entries(file_id));
        }
        FUN | SPEC_FUN | SPEC_INLINE_FUN => {
            let fun = scope.syntax_cast::<ast::AnyFun>().unwrap();
            entries.extend(fun.value.params_as_bindings().to_in_file_entries(file_id));
        }
        LAMBDA_EXPR => {
            let lambda_expr = scope.syntax_cast::<ast::LambdaExpr>().unwrap();
            entries.extend(lambda_expr.value.param_ident_pats().to_in_file_entries(file_id));
        }
        SCHEMA => {
            let schema = scope.syntax_cast::<ast::Schema>().unwrap();
            entries.extend(
                schema
                    .value
                    .schema_fields_as_bindings()
                    .to_in_file_entries(file_id),
            )
        }
        FOR_EXPR => {
            let for_expr = scope.syntax_cast::<ast::ForExpr>().unwrap();
            let idx_binding = for_expr.value.for_condition().and_then(|it| it.ident_pat());
            if let Some(idx_binding) = idx_binding {
                entries.extend(InFile::new(file_id, idx_binding).to_entry())
            }
        }
        _ => {}
    }

    entries
}

fn builtin_functions(db: &dyn SourceDatabase) -> Vec<InFile<ast::Fun>> {
    builtin_module(db)
        .map(|module| module.map(|it| it.functions()).flatten())
        .unwrap_or_default()
}

fn builtin_spec_functions(db: &dyn SourceDatabase) -> Vec<InFile<ast::SpecFun>> {
    builtin_module(db)
        .map(|module| module.map(|it| it.spec_functions()).flatten())
        .unwrap_or_default()
}

fn builtin_module(db: &dyn SourceDatabase) -> Option<InFile<ast::Module>> {
    let file_id = match db.builtins_file_id() {
        Some(fid) => fid,
        None => {
            tracing::error!("builtins_file is not set");
            return None;
        }
    };
    let builtins_module = db
        .parse(file_id)
        .tree()
        .modules()
        .collect::<Vec<_>>()
        .pop()
        .expect("0x0::builtins");
    Some(builtins_module.in_file(file_id))
}
