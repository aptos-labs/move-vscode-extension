use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::ast::NamedItemScope;
use crate::{ast, AstNode, SyntaxNode};
use parser::SyntaxKind::SCHEMA;

pub trait MoveSyntaxNodeExt {
    fn containing_module(&self) -> Option<ast::Module>;
    fn containing_file(&self) -> Option<ast::SourceFile>;
    fn item_scope(&self) -> NamedItemScope;
}

impl MoveSyntaxNodeExt for SyntaxNode {
    fn containing_module(&self) -> Option<ast::Module> {
        self.ancestor_strict::<ast::Module>()
    }

    fn containing_file(&self) -> Option<ast::SourceFile> {
        self.ancestor_strict::<ast::SourceFile>()
    }

    fn item_scope(&self) -> NamedItemScope {
        use crate::SyntaxKind::*;

        let ancestors = self.ancestors();
        for ancestor in ancestors {
            let Some(has_attrs) = ast::AnyHasAttrs::cast(ancestor.clone()) else {
                continue;
            };
            if matches!(
                ancestor.kind(),
                SCHEMA | ITEM_SPEC | MODULE_SPEC | SPEC_BLOCK_EXPR
            ) {
                return NamedItemScope::Verify;
            }
            if let Some(ancestor_scope) = item_scope_from_attributes(has_attrs) {
                return ancestor_scope;
            }
        }
        NamedItemScope::Main
    }
}

fn item_scope_from_attributes(has_attrs: impl ast::HasAttrs) -> Option<NamedItemScope> {
    if has_attrs.has_atom_attr("test_only") || has_attrs.has_atom_attr("test") {
        return Some(NamedItemScope::Test);
    }
    if has_attrs.has_atom_attr("verify_only") {
        return Some(NamedItemScope::Verify);
    }
    None
}
