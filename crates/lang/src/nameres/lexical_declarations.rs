use crate::member_items::HasMembersList;
use crate::nameres::namespaces::{ALL_NS, NAMES, TYPES};
use crate::nameres::paths::ResolutionContext;
use crate::nameres::processors::{ProcessingStatus, Processor};
use crate::nameres::scope::ScopeEntry;
use syntax::algo::ComparePos;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::ast::{AnyHasTypeParams, HasItemList, HasStmtList, HasTypeParams, IdentPat};
use syntax::{algo, ast, match_ast, AstNode};

pub fn process_nested_scopes_upwards(
    ctx: ResolutionContext,
    processor: &impl Processor,
) -> ProcessingStatus {
    let mut next_scope = ctx.path.syntax().parent();
    while let Some(scope) = next_scope {
        if AnyHasTypeParams::can_cast(scope.kind()) {
            let type_params_owner = AnyHasTypeParams::cast(scope.clone()).unwrap();
            let stop = processor.process_all_named(type_params_owner.type_params(), TYPES);
            if stop.is_stop() {
                return stop;
            }
        }
        let stop = match_ast! {
            match scope {
                ast::Module(it) => process_module_scope(it, processor),
                ast::Fun(it) => {
                    processor.process_all_named(it.params_as_bindings(), NAMES)
                },
                ast::BlockExpr(it) => process_any_block_expr_scope(it.into(), ctx.clone(), processor),
                _ => ProcessingStatus::Continue
            }
        };
        if stop.is_stop() {
            break;
        }
        next_scope = scope.parent();
    }
    ProcessingStatus::Continue
}

fn process_module_scope(module: ast::Module, processor: &impl Processor) -> ProcessingStatus {
    for (member_items, ns) in module.member_items_with_ns() {
        if processor.process_all_named(member_items, ns).is_stop() {
            return ProcessingStatus::Stop;
        }
    }

    // for use_speck in module.use_specks() {
    //     if let Some(path) = use_speck.path() {
    //         if let Some(name_ref) = path.name_ref() {
    //             let stop = processor.process(ScopeEntry::from_name_ref(name_ref, ALL_NS));
    //             if stop.is_stop() {
    //                 return stop;
    //             }
    //         }
    //     }
    // }

    ProcessingStatus::Continue
}

fn process_any_block_expr_scope(
    any_block_expr: ast::AnyHasStmtList,
    ctx: ResolutionContext,
    processor: &impl Processor,
) -> ProcessingStatus {
    let mut visible_let_stmts = any_block_expr
        .let_stmts()
        .filter(|let_stmt| {
            let prev = ctx.path.syntax();
            // if `prev` before `let_stmt`, then `let_stmt` is not visible
            if algo::compare_by_position(prev, let_stmt.syntax()) == ComparePos::Before {
                return false;
            }
            // drops let-statement that is ancestors of ref (on the same statement, at most one)
            if prev == let_stmt.syntax() || let_stmt.syntax().is_ancestor_of(prev) {
                return false;
            }
            true
        })
        .collect::<Vec<_>>();
    // shadowing support (look at latest first)
    visible_let_stmts.reverse();

    let ident_pats = visible_let_stmts
        .iter()
        .flat_map(|let_stmt| let_stmt.syntax().descendants_of_type::<IdentPat>())
        .collect::<Vec<_>>();
    for ident_pat in ident_pats {
        let stop = processor.process_named(ident_pat, NAMES);
        if stop.is_stop() {
            return stop;
        }
    }

    ProcessingStatus::Continue
}
