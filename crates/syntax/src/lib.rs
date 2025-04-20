#![allow(unused)]

pub mod algo;
pub mod ast;
pub mod files;
mod parsing;
mod ptr;
pub mod syntax_editor;
mod syntax_error;
mod syntax_node;
mod ted;
mod token_text;
mod validation;

pub use crate::{
    ast::{AstNode, AstToken},
    ptr::{AstPtr, SyntaxNodePtr},
    syntax_error::SyntaxError,
    syntax_node::{
        IntoNodeOrToken, PreorderWithTokens, SyntaxElement, SyntaxElementChildren, SyntaxNode,
        SyntaxNodeChildren, SyntaxNodeOrToken, SyntaxToken, SyntaxTreeBuilder,
    },
    token_text::TokenText,
};
use parser::{entry_points, Parser};
pub use parser::{SyntaxKind, T};
pub use rowan::{
    Direction, GreenNode, NodeOrToken, SyntaxText, TextRange, TextSize, TokenAtOffset, WalkEvent,
};
use std::ops::Range;
use std::sync::Arc;

/// `Parse` is the result of the parsing: a syntax tree and a collection of
/// errors.
///
/// Note that we always produce a syntax tree, even for completely invalid
/// files.
#[derive(Debug, PartialEq, Eq)]
pub struct Parse {
    green: GreenNode,
    errors: Arc<Vec<SyntaxError>>,
}

impl Clone for Parse {
    fn clone(&self) -> Parse {
        Parse {
            green: self.green.clone(),
            errors: self.errors.clone(),
        }
    }
}

impl Parse {
    fn new(green: GreenNode, errors: Vec<SyntaxError>) -> Parse {
        Parse {
            green,
            errors: Arc::new(errors),
        }
    }

    pub fn syntax_node(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green.clone())
    }
    pub fn errors(&self) -> Vec<SyntaxError> {
        let mut errors = self.errors.to_vec();
        validation::validate(&self.syntax_node(), &mut errors);
        errors
    }
}

impl Parse {
    /// Converts this parse result into a parse result for an untyped syntax tree.
    pub fn to_syntax(self) -> Parse {
        Parse {
            green: self.green,
            errors: self.errors,
        }
    }

    /// Gets the parsed syntax tree as a typed ast node.
    ///
    /// # Panics
    ///
    /// Panics if the root node cannot be casted into the typed ast node
    /// (e.g. if it's an `ERROR` node).
    pub fn tree(&self) -> SourceFile {
        SourceFile::cast(self.syntax_node()).unwrap()
    }

    /// Converts from `Parse<T>` to [`Result<T, Vec<SyntaxError>>`].
    pub fn ok(self) -> Result<SourceFile, Vec<SyntaxError>> {
        match self.errors() {
            errors if !errors.is_empty() => Err(errors.to_vec()),
            _ => Ok(self.tree()),
        }
    }

    pub fn reparse(&self, delete: TextRange, insert: &str) -> Parse {
        // self.incremental_reparse(delete, insert, edition)
        //     .unwrap_or_else(|| self.full_reparse(delete, insert, edition))
        self.full_reparse(delete, insert)
    }

    fn full_reparse(&self, delete: TextRange, insert: &str) -> Parse {
        let mut text = self.tree().syntax().text().to_string();
        text.replace_range(Range::<usize>::from(delete), insert);
        SourceFile::parse(&text)
    }
}

/// `SourceFile` represents a parse tree for a single Rust file.
pub use crate::ast::SourceFile;

pub fn parse_with_entrypoint(text: &str, entrypoint: fn(&mut Parser)) -> Parse {
    let (green, errors) = parsing::parse_text(text, entrypoint);
    Parse {
        green,
        errors: Arc::new(errors),
    }
}

impl SourceFile {
    pub fn parse(text: &str) -> Parse {
        let (green, mut errors) = parsing::parse_text(text, entry_points::source_file);
        let root = SyntaxNode::new_root(green.clone());

        assert_eq!(root.kind(), SyntaxKind::SOURCE_FILE);
        Parse {
            green,
            errors: Arc::new(errors),
        }
    }
}

/// Matches a `SyntaxNode` against an `ast` type.
///
/// # Example:
///
/// ```ignore
/// match_ast! {
///     match node {
///         ast::CallExpr(it) => { ... },
///         ast::MethodCallExpr(it) => { ... },
///         ast::MacroCall(it) => { ... },
///         _ => None,
///     }
/// }
/// ```
#[macro_export]
macro_rules! match_ast {
    (match $node:ident { $($tt:tt)* }) => { $crate::match_ast!(match ($node) { $($tt)* }) };

    (match ($node:expr) {
        $( $( $path:ident )::+ ($it:pat) => $res:expr, )*
        _ => $catch_all:expr $(,)?
    }) => {{
        $( if let Some($it) = $($path::)+cast($node.clone()) { $res } else )*
        { $catch_all }
    }};
}
