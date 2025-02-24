mod lexer;
mod text_token_source;
mod text_tree_sink;

use crate::SyntaxError;
use parser::{entry_points, Parser};
use rowan::GreenNode;

pub(crate) use crate::parsing::lexer::*;
use crate::parsing::text_token_source::TextTokenSource;
use crate::parsing::text_tree_sink::TextTreeSink;

pub(crate) fn parse_text(text: &str, entry_point: fn(&mut Parser)) -> (GreenNode, Vec<SyntaxError>) {
    let (tokens, lexer_errors) = tokenize(text);

    let mut token_source = TextTokenSource::new(text, &tokens);
    let mut tree_sink = TextTreeSink::new(text, &tokens);

    parser::parse(&mut token_source, &mut tree_sink, entry_point);

    let (tree, mut parser_errors) = tree_sink.finish();
    parser_errors.extend(lexer_errors);

    (tree, parser_errors)
}
