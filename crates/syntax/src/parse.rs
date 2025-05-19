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

/// `TokenSource` abstracts the source of the tokens parser operates on.
///
/// Hopefully this will allow us to treat text and token trees in the same way!
pub trait TokenSource {
    fn current(&self) -> Token;

    /// Lookahead n token
    fn lookahead_nth(&self, n: usize) -> Token;

    /// bump cursor to next token
    fn bump(&mut self);

    /// rollback to the previous token
    fn rollback(&mut self);

    /// Is the current token a specified keyword?
    fn is_keyword(&self, kw: &str) -> bool;
}

/// `Token` abstracts the cursor of `TokenSource` operates on.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Token {
    /// What is the current token?
    pub kind: SyntaxKind,

    /// Is the current token joined to the next one (`> >` vs `>>`).
    pub is_jointed_to_next: bool,
}

/// `TreeSink` abstracts details of a particular syntax tree implementation.
pub trait TreeSink {
    /// Adds new token to the current branch.
    fn token(&mut self, kind: SyntaxKind, n_tokens: u8);

    /// Start new branch and make it current.
    fn start_node(&mut self, kind: SyntaxKind);

    /// Finish current branch and restore previous
    /// branch as current.
    fn finish_node(&mut self);

    fn error(&mut self, error: ParseError);
}

pub fn parse(
    token_source: &mut dyn TokenSource,
    tree_sink: &mut dyn TreeSink,
    entry_point: fn(&mut Parser),
) {
    let mut p = Parser::new(token_source);
    entry_point(&mut p);
    let events = p.finish();
    event::process(tree_sink, events);
}

pub(crate) fn parse_text(text: &str, entry_point: fn(&mut Parser)) -> (GreenNode, Vec<SyntaxError>) {
    let (tokens, lexer_errors) = tokenize(text);

    let mut token_source = TextTokenSource::new(text, &tokens);
    let mut tree_sink = TextTreeSink::new(text, &tokens);

    parse(&mut token_source, &mut tree_sink, entry_point);

    let (tree, mut parser_errors) = tree_sink.finish();
    parser_errors.extend(lexer_errors);

    (tree, parser_errors)
}
