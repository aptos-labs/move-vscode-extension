mod event;
mod grammar;
pub(crate) mod parser;
mod token_set;

mod lexer;
pub mod move_model_lexer;
mod text_token_source;
mod text_tree_sink;

use crate::parse::lexer::tokenize;
use crate::parse::parser::Parser;
use crate::parse::text_token_source::TextTokenSource;
use crate::parse::text_tree_sink::TextTreeSink;
use crate::SyntaxError;
use rowan::GreenNode;

pub use crate::syntax_kind::SyntaxKind;
pub use grammar::entry_points;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParseError(pub Box<String>);

/// `Token` abstracts the cursor of `TokenSource` operates on.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Token {
    /// What is the current token?
    pub kind: SyntaxKind,

    /// Is the current token joined to the next one (`> >` vs `>>`).
    pub is_jointed_to_next: bool,
}

pub(crate) fn parse_text(text: &str, entry_point: fn(&mut Parser)) -> (GreenNode, Vec<SyntaxError>) {
    let (tokens, lexer_errors) = tokenize(text);

    let mut token_source = TextTokenSource::new(text, &tokens);

    let mut p = Parser::new(&mut token_source);
    entry_point(&mut p);
    let events = p.finish();

    let mut tree_sink = TextTreeSink::new(text, &tokens);
    event::process(&mut tree_sink, events);

    let (tree, mut parser_errors) = tree_sink.finish();
    parser_errors.extend(lexer_errors);

    (tree, parser_errors)
}
