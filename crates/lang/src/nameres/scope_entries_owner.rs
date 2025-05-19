use crate::hir_db;
use crate::loc::SyntaxLocFileExt;
use crate::nameres::blocks::get_entries_in_blocks;
use crate::nameres::get_schema_field_entries;
use crate::nameres::scope::{NamedItemsExt, NamedItemsInFileExt, ScopeEntry, ScopeEntryExt};
use crate::node_ext::item_spec::ItemSpecExt;
use base_db::{SourceDatabase, source_db};
use syntax::ast::{FieldsOwner, GenericElement, HasItems};
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, SyntaxNode, ast, match_ast};

pub fn get_entries_in_scope(
    db: &dyn SourceDatabase,
    scope: InFile<SyntaxNode>,
    prev: SyntaxNode,
) -> Vec<ScopeEntry> {
    let mut entries = vec![];
    if let Some(use_stmts_owner) = scope.syntax_cast::<ast::AnyHasUseStmts>() {
        entries.extend(hir_db::use_speck_entries(db, use_stmts_owner));
    }
    entries.extend(get_entries_in_blocks(scope.clone(), prev));
    entries.extend(get_entries_from_owner(db, scope));
    entries
}

pub fn get_entries_from_owner(db: &dyn SourceDatabase, scope: InFile<SyntaxNode>) -> Vec<ScopeEntry> {
    use syntax::SyntaxKind::*;

    let file_id = scope.file_id;
    let mut entries = vec![];

    if let Some(has_type_params) = ast::AnyGenericElement::cast(scope.value.clone()) {
        entries.extend(has_type_params.type_params().to_entries(file_id));
    }

    match scope.value.kind() {
        MODULE => {
            let module = scope.syntax_cast::<ast::Module>().unwrap();
            entries.extend(hir_db::module_importable_entries(db, module.loc()));

            entries.extend(module.value.enum_variants().to_entries(file_id));

            entries.extend(builtin_functions(db).to_entries());
            entries.extend(builtin_spec_functions(db).to_entries());
            entries.extend(builtin_spec_consts(db).to_entries());
        }
        MODULE_SPEC => {
            let module_spec = scope.syntax_cast::<ast::ModuleSpec>().unwrap();

            let importable_entries = module_spec.flat_map(|it| it.importable_items()).to_entries();
            entries.extend(importable_entries);

            entries.extend(builtin_spec_functions(db).to_entries());
        }
        ITEM_SPEC => {
            let item_spec = scope.syntax_cast::<ast::ItemSpec>().unwrap();
            if let Some(item) = item_spec.item(db) {
                let (fid, item) = item.unpack();
                match_ast! {
                    match (item.syntax()) {
                        ast::Fun(fun) => {
                            let any_fun = fun.clone().to_any_fun();
                            entries.extend(any_fun.type_params().to_entries(fid));
                            entries.extend(any_fun.params_as_bindings().to_entries(fid));
                        },
                        ast::Struct(struct_) => {
                            entries.extend(struct_.named_fields().to_entries(fid));
                        },
                        _ => ()
                    }
                }
            }
        }
        // todo: ITEM_SPEC should have access to params / fields of the item
        SCRIPT => {
            let script = scope.syntax_cast::<ast::Script>().unwrap();
            entries.extend(script.value.consts().to_entries(file_id));
        }
        FUN | SPEC_FUN | SPEC_INLINE_FUN => {
            let fun = scope.syntax_cast::<ast::AnyFun>().unwrap();
            entries.extend(fun.value.params_as_bindings().to_entries(file_id));
        }
        LAMBDA_EXPR => {
            let lambda_expr = scope.syntax_cast::<ast::LambdaExpr>().unwrap();
            entries.extend(lambda_expr.value.param_ident_pats().to_entries(file_id));
        }
        SCHEMA => {
            let schema = scope.syntax_cast::<ast::Schema>().unwrap();
            let schema_field_entries = get_schema_field_entries(schema);
            entries.extend(schema_field_entries);
        }
        FOR_EXPR => {
            let for_expr = scope.syntax_cast::<ast::ForExpr>().unwrap();
            let idx_binding = for_expr.value.for_condition().and_then(|it| it.ident_pat());
            if let Some(idx_binding) = idx_binding {
                entries.extend(InFile::new(file_id, idx_binding).to_entry())
            }
        }
        FORALL_EXPR | EXISTS_EXPR | CHOOSE_EXPR => {
            let owner = scope.syntax_cast::<ast::QuantBindingsOwner>().unwrap();
            entries.extend(
                owner
                    .value
                    .quant_bindings_as_ident_pats()
                    .to_entries(owner.file_id),
            );
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

fn builtin_spec_consts(db: &dyn SourceDatabase) -> Vec<InFile<ast::Const>> {
    builtin_module(db)
        .map(|module| module.map(|it| it.consts()).flatten())
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
    let builtins_module = source_db::parse(db, file_id)
        .tree()
        .modules()
        .collect::<Vec<_>>()
        .pop()
        .expect("0x0::builtins");
    Some(builtins_module.in_file(file_id.data(db)))
}
