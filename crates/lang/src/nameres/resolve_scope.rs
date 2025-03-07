use crate::nameres::node_ext::ModuleResolutionExt;
use crate::nameres::scope::{NamedItemsExt, ScopeEntry};
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::ast::{HasItemList, HasStmtList, HasTypeParams};
use syntax::{ast, AstNode};

pub enum ResolveScope {
    Module(ast::Module),
    Script(ast::Script),
    Fun(ast::Fun),
    Schema(ast::Schema),
    ModuleSpec(ast::ModuleSpec),
}

impl ResolveScope {
    pub fn scope_entries(&self) -> Vec<ScopeEntry> {
        use crate::nameres::resolve_scope::ResolveScope::*;

        let mut entries = vec![];
        match self {
            Module(m) => {
                entries.extend(m.member_entries());
                entries.extend(m.enum_variants().to_entries());
            }
            Script(s) => {
                entries.extend(s.consts().to_entries());
            }
            Fun(f) => {
                entries.extend(f.type_params().to_entries());
                entries.extend(f.params_as_bindings().to_entries());
            }
            Schema(s) => {
                entries.extend(s.schema_fields_as_bindings().to_entries())
            }
            ModuleSpec(module_spec) => {
                entries.extend(module_spec.spec_functions().to_entries());
                entries.extend(module_spec.spec_inline_functions().to_entries());
                entries.extend(module_spec.schemas().to_entries());
            }
        }
        entries
    }
}

fn visible_let_stmts(
    block: ast::BlockExpr,
    currently_at: ast::Stmt,
) -> Vec<(ast::LetStmt, Vec<ScopeEntry>)> {
    block
        .let_stmts()
        .filter(|let_stmt| let_stmt.syntax().strictly_before(currently_at.syntax()))
        .map(|let_stmt| {
            let bindings = let_stmt.pat().map(|pat| pat.bindings()).unwrap_or_default();
            (let_stmt, bindings.to_entries())
        })
        .collect()
}
