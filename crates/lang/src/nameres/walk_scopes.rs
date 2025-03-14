use crate::member_items::HasMembersList;
use crate::nameres::namespaces::NAMES;
use crate::nameres::processors::{ProcessingStatus, Processor};
use parser::SyntaxKind::STMT_LIST;
use syntax::algo::ComparePos;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::ast::{HasStmtList, IdentPat};
use syntax::{algo, ast, match_ast, AstNode, SyntaxNode, SyntaxToken};

pub fn walk_scopes(start_at: SyntaxToken, processor: &impl Processor) -> Option<()> {
    let mut opt_scope = start_at.parent();
    let mut came_from = None;
    while let Some(scope) = opt_scope {
        let parent_scope = scope.parent();
        if process_scope(scope.clone(), came_from.clone(), processor).is_stop() {
            break;
        }
        // skip StmtList to be able to use came_from in let stmts shadowing
        if scope.kind() != STMT_LIST {
            came_from = Some(scope);
        }
        opt_scope = parent_scope;
    }
    None
}

fn process_scope(
    scope: SyntaxNode,
    came_from: Option<SyntaxNode>,
    processor: &impl Processor,
) -> ProcessingStatus {
    match_ast! {
        match scope {
            ast::BlockExpr(it) => process_any_block_expr_scope(it.into(), came_from, processor),
            ast::Fun(it) => {
                processor.process_all_named(it.params_as_bindings(), NAMES)
            },
            ast::Module(it) => process_module_scope(it, processor),
            _ => ProcessingStatus::Continue
        }
    }
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
    previous_scope: Option<SyntaxNode>,
    processor: &impl Processor,
) -> ProcessingStatus {
    let mut visible_let_stmts = any_block_expr
        .let_stmts()
        .filter(|let_stmt| {
            if let Some(prev) = &previous_scope {
                // if `prev` before `let_stmt`, then `let_stmt` is not visible
                if algo::compare_by_position(prev, let_stmt.syntax()) == ComparePos::Before {
                    return false;
                }
                // drops let-statement that is ancestors of ref (on the same statement, at most one)
                if prev == let_stmt.syntax() || let_stmt.syntax().is_ancestor_of(prev) {
                    return false;
                }
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
