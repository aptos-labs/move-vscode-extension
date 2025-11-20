// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::hir_db;
use crate::loc::{SyntaxLoc, SyntaxLocFileExt, SyntaxLocInput};
use crate::nameres::get_schema_field_entries;
use crate::nameres::namespaces::Ns;
use crate::nameres::scope::{NamedItemsExt, NamedItemsInFileExt, ScopeEntry, ScopeEntryExt};
use crate::node_ext::item_spec::ItemSpecExt;
use base_db::{SourceDatabase, source_db};
use std::collections::HashSet;
use syntax::ast::HasItems;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, SyntaxNode, ast};

pub fn get_entries_in_scope<'db>(
    db: &'db dyn SourceDatabase,
    scope: &InFile<SyntaxNode>,
) -> &'db Vec<ScopeEntry> {
    let scope_loc = SyntaxLocInput::new(db, SyntaxLoc::from_file_syntax_node(scope));
    get_entries_in_scope_tracked(db, scope_loc)
}

#[salsa_macros::tracked(returns(ref))]
fn get_entries_in_scope_tracked(
    db: &dyn SourceDatabase,
    scope_loc: SyntaxLocInput<'_>,
) -> Vec<ScopeEntry> {
    let Some(scope) = scope_loc.syntax_loc(db).to_syntax_node(db) else {
        return vec![];
    };

    let mut entries = vec![];
    if let Some(use_stmts_owner) = scope.syntax_cast::<ast::AnyUseStmtsOwner>() {
        entries.extend(hir_db::use_speck_entries(db, &use_stmts_owner));
    }
    entries.extend(get_entries_from_owner(db, &scope));

    entries
}

pub fn get_entries_from_owner(db: &dyn SourceDatabase, scope: &InFile<SyntaxNode>) -> Vec<ScopeEntry> {
    use syntax::SyntaxKind::*;

    let file_id = scope.file_id;
    let mut entries = vec![];

    if let Some(generic_element) = ast::GenericElement::cast(scope.value.clone()) {
        entries.extend(generic_element.type_params().to_entries(file_id));
    }

    if scope.value.is_msl_only_scope() {
        entries.extend(builtin_spec_functions(db).to_entries());
    }

    match scope.value.kind() {
        MODULE => {
            let module = scope.syntax_cast::<ast::Module>().unwrap();
            entries.extend(hir_db::module_importable_entries(db, module.loc()));
            entries.extend(module.value.enum_variants().to_entries(file_id));
            entries.extend(builtin_functions(db).to_entries());
        }
        MODULE_SPEC => {
            let (file_id, module_spec) = scope.syntax_cast::<ast::ModuleSpec>().unwrap().unpack();
            entries.extend(module_spec.importable_items().to_entries(file_id));
        }
        SCRIPT => {
            let script = scope.syntax_cast::<ast::Script>().unwrap();
            let consts = script.flat_map(|it| it.consts()).to_entries();
            entries.extend(consts);
        }
        ITEM_SPEC => {
            let item_spec = scope.syntax_cast::<ast::ItemSpec>().unwrap();
            if let Some(item) = item_spec.item(db) {
                let (fid, item) = item.unpack();
                match item {
                    ast::ItemSpecItem::Fun(fun) => {
                        let fun = fun.to_any_fun();
                        entries.extend(fun.to_generic_element().type_params().to_entries(fid));
                        entries.extend(fun.params_as_bindings().to_entries(fid));
                    }
                    ast::ItemSpecItem::StructOrEnum(struct_or_enum) => {
                        entries.extend(struct_or_enum.named_fields().to_entries(fid));
                    }
                }
            }
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
        FORALL_EXPR | EXISTS_EXPR => {
            let owner = scope.syntax_cast::<ast::QuantExpr>().unwrap();
            entries.extend(
                owner
                    .value
                    .quant_bindings_as_ident_pats()
                    .to_entries(owner.file_id),
            );
        }
        CHOOSE_EXPR => {
            let choose_expr = scope.syntax_cast::<ast::ChooseExpr>().unwrap();
            if let Some(ident_pat) = choose_expr.value.quant_binding_ident_pat() {
                entries.extend(ident_pat.in_file(file_id).to_entry());
            }
        }
        AXIOM_STMT | INVARIANT_STMT => {
            let generic_stmt = scope.syntax_cast::<ast::GenericSpecStmt>().unwrap();
            entries.extend(generic_stmt.value.type_params().to_entries(generic_stmt.file_id))
        }
        APPLY_SCHEMA => {
            let (file_id, apply_schema) = scope.syntax_cast::<ast::ApplySchema>().unwrap().unpack();
            for wildcard in apply_schema.apply_to_patterns() {
                entries.extend(wildcard.type_params().to_entries(file_id));
            }
        }
        MATCH_ARM => {
            // coming from rhs, use pat bindings from lhs
            let (file_id, match_arm) = scope.syntax_cast::<ast::MatchArm>().unwrap().unpack();
            let ident_pats = match_arm.pat().map(|it| it.bindings()).unwrap_or_default();
            entries.extend(ident_pats.to_entries(file_id));
        }
        _ => (),
    }

    if matches!(scope.value.kind(), MODULE | SCRIPT) {
        let const_entries = entries
            .iter()
            .filter(|it| it.ns == Ns::NAME)
            .map(|it| it.name.clone())
            .collect::<HashSet<_>>();
        for builtin_const in builtin_consts(db) {
            if builtin_const
                .value
                .name()
                .is_some_and(|it| const_entries.contains(&it.as_string()))
            {
                continue;
            }
            entries.extend(builtin_const.to_entry());
        }
    }
    // entries.extend(builtin_consts(db).to_entries());

    entries
}

fn builtin_functions(db: &dyn SourceDatabase) -> Vec<InFile<ast::Fun>> {
    builtin_module(db)
        .map(|module| module.map(|it| it.functions()).flatten())
        .unwrap_or_default()
}

fn builtin_consts(db: &dyn SourceDatabase) -> Vec<InFile<ast::Const>> {
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
