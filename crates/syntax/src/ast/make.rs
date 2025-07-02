pub(crate) mod quote;

use crate::ast::make::quote::quote;
use crate::parse::SyntaxKind;
use crate::{AstNode, SourceFile, SyntaxToken, ast};

// // pub fn name(name: &str) -> ast::Name {
// //     ast_from_text(&format!("module {name}"))
// // }
// pub fn name_ref(name_ref: &str) -> ast::NameRef {
//     quote! {
//         NameRef {
//             [IDENT format!("{name_ref}")]
//         }
//     }
// }

// fn ty_from_text(text: &str) -> ast::Type {
//     ast_from_text(&format!("module 0x1::m {{ const M: {}; }}", text))
// }

// fn expr_from_text<E: Into<ast::Expr> + AstNode>(text: &str) -> E {
//     ast_from_text(&format!("module 0x1::m {{ fun main() {{ {} }} }}", text))
// }

#[track_caller]
fn ast_from_text<N: AstNode>(text: &str) -> N {
    let parse = SourceFile::parse(text);
    let node = match parse.tree().syntax().descendants().find_map(N::cast) {
        Some(it) => it,
        None => {
            let node = std::any::type_name::<N>();
            panic!("Failed to make ast node `{node}` from text {text}")
        }
    };
    let node = node.clone_subtree();
    assert_eq!(node.syntax().text_range().start(), 0.into());
    node
}

// pub fn token(kind: SyntaxKind) -> SyntaxToken {
//     tokens::SOURCE_FILE
//         .tree()
//         .syntax()
//         .clone_for_update()
//         .descendants_with_tokens()
//         .filter_map(|it| it.into_token())
//         .find(|it| it.kind() == kind)
//         .unwrap_or_else(|| panic!("unhandled token: {kind:?}"))
// }

pub mod tokens {
    use std::sync::LazyLock;

    use crate::{AstNode, Parse, SourceFile, SyntaxKind::*, SyntaxToken, ast};

    pub(crate) static SOURCE_FILE: LazyLock<Parse> =
        LazyLock::new(|| SourceFile::parse("module 0x1::m { fun main() { 1; 1 + 1; } }"));

    pub fn semicolon() -> SyntaxToken {
        SOURCE_FILE
            .tree()
            .syntax()
            .clone_for_update()
            .descendants_with_tokens()
            .filter_map(|it| it.into_token())
            .find(|it| it.kind() == SEMICOLON)
            .unwrap()
    }

    pub fn single_space() -> SyntaxToken {
        SOURCE_FILE
            .tree()
            .syntax()
            .clone_for_update()
            .descendants_with_tokens()
            .filter_map(|it| it.into_token())
            .find(|it| it.kind() == WHITESPACE && it.text() == " ")
            .unwrap()
    }

    pub fn whitespace(text: &str) -> SyntaxToken {
        assert!(text.trim().is_empty());
        let sf = SourceFile::parse(text).ok().unwrap();
        sf.syntax()
            .clone_for_update()
            .first_child_or_token()
            .unwrap()
            .into_token()
            .unwrap()
    }

    pub fn doc_comment(text: &str) -> SyntaxToken {
        assert!(!text.trim().is_empty());
        let sf = SourceFile::parse(text).ok().unwrap();
        sf.syntax().first_child_or_token().unwrap().into_token().unwrap()
    }

    // pub fn literal(text: &str) -> SyntaxToken {
    //     assert_eq!(text.trim(), text);
    //     let lit: ast::Literal = super::ast_from_text(&format!("fn f() {{ let _ = {text}; }}"));
    //     lit.syntax().first_child_or_token().unwrap().into_token().unwrap()
    // }

    // pub fn ident(text: &str) -> SyntaxToken {
    //     assert_eq!(text.trim(), text);
    //     let path: ast::Path = super::ext::ident_path(text);
    //     path.syntax()
    //         .descendants_with_tokens()
    //         .filter_map(|it| it.into_token())
    //         .find(|it| it.kind() == IDENT)
    //         .unwrap()
    // }

    pub fn single_newline() -> SyntaxToken {
        let res = SOURCE_FILE
            .tree()
            .syntax()
            .clone_for_update()
            .descendants_with_tokens()
            .filter_map(|it| it.into_token())
            .find(|it| it.kind() == WHITESPACE && it.text() == "\n")
            .unwrap();
        res.detach();
        res
    }

    pub fn blank_line() -> SyntaxToken {
        SOURCE_FILE
            .tree()
            .syntax()
            .clone_for_update()
            .descendants_with_tokens()
            .filter_map(|it| it.into_token())
            .find(|it| it.kind() == WHITESPACE && it.text() == "\n\n")
            .unwrap()
    }
}
