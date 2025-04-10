use super::{ast_from_text, SyntaxFactory};
use crate::ast::make::quote::quote;
use crate::{
    ast::{self, make},
    AstNode, SourceFile, SyntaxNode, SyntaxToken,
};
use parser::SyntaxKind;
use stdx::itertools::Itertools;

impl SyntaxFactory {
    pub fn arg_list(&self, args: impl IntoIterator<Item = ast::Expr>) -> ast::ArgList {
        let (args, input) = iterator_input(args);
        let args = args.into_iter().format(", ");
        ast_from_text(&format!("module 0x1::m {{ fun main() {{ call({args}) }} }}"))
    }

    pub fn name(&self, name: &str) -> ast::Name {
        ast_from_text::<ast::Name>(&format!("module {name}")).clone_for_update()
    }

    pub fn name_ref(&self, name_ref: &str) -> ast::NameRef {
        quote! {
            NameRef {
                [IDENT format!("{name_ref}")]
            }
        }
        .clone_for_update()
    }

    pub fn token(&self, kind: SyntaxKind) -> SyntaxToken {
        make::tokens::SOURCE_FILE
            .tree()
            .syntax()
            .clone_for_update()
            .descendants_with_tokens()
            .filter_map(|it| it.into_token())
            .find(|it| it.kind() == kind)
            .unwrap_or_else(|| panic!("unhandled token: {kind:?}"))
    }

    pub fn whitespace(&self, text: &str) -> SyntaxToken {
        assert!(text.trim().is_empty());
        let sf = SourceFile::parse(text).ok().unwrap();
        sf.syntax()
            .clone_for_update()
            .first_child_or_token()
            .unwrap()
            .into_token()
            .unwrap()
    }
}

// We need to collect `input` here instead of taking `impl IntoIterator + Clone`,
// because if we took `impl IntoIterator + Clone`, that could be something like an
// `Iterator::map` with a closure that also makes use of a `SyntaxFactory` constructor.
//
// In that case, the iterator would be evaluated inside of the call to `map_children`,
// and the inner constructor would try to take a mutable borrow of the mappings `RefCell`,
// which would panic since it's already being mutably borrowed in the outer constructor.
pub(crate) fn iterator_input<N: AstNode>(input: impl IntoIterator<Item = N>) -> (Vec<N>, Vec<SyntaxNode>) {
    input
        .into_iter()
        .map(|it| {
            let syntax = it.syntax().clone();
            (it, syntax)
        })
        .collect()
}
