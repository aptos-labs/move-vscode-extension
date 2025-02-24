use crate::parser::Marker;
use crate::token_set::TokenSet;
use crate::SyntaxKind::{EOF, ERROR, FUN, IDENT};
use crate::{Parser, SyntaxKind, T};

/// The `parser` passed this is required to at least consume one token if it returns `true`.
/// If the `parser` returns false, parsing will stop.
pub(crate) fn list(
    p: &mut Parser<'_>,
    bra: SyntaxKind,
    ket: SyntaxKind,
    delim: SyntaxKind,
    unexpected_delim_message: impl Fn() -> String,
    item_first_set: TokenSet,
    mut parser: impl FnMut(&mut Parser<'_>) -> bool,
) {
    p.bump(bra);
    delimited(
        p,
        delim,
        unexpected_delim_message,
        |p| p.at(ket),
        item_first_set,
        parser,
    );
    p.expect(ket);
}

pub(crate) fn comma_separated_list(
    p: &mut Parser,
    unexpected_delim_message: &str,
    is_end: impl Fn(&Parser) -> bool,
    item_first_set: TokenSet,
    mut parser: impl FnMut(&mut Parser<'_>) -> bool,
) {
    delimited(
        p,
        T![,],
        || unexpected_delim_message.into(),
        is_end,
        item_first_set,
        parser,
    )
}

pub(crate) fn delimited(
    p: &mut Parser,
    delim: SyntaxKind,
    unexpected_delim_message: impl Fn() -> String,
    is_end: impl Fn(&Parser) -> bool,
    item_first_set: TokenSet,
    mut parser: impl FnMut(&mut Parser<'_>) -> bool,
) {
    delimited_fn(
        p,
        delim,
        unexpected_delim_message,
        is_end,
        |p| p.at_ts(item_first_set),
        parser,
    );
}

pub(crate) fn delimited_fn(
    p: &mut Parser,
    delim: SyntaxKind,
    unexpected_delim_message: impl Fn() -> String,
    is_end: impl Fn(&Parser) -> bool,
    at_item_first: impl Fn(&Parser) -> bool,
    mut parser: impl FnMut(&mut Parser<'_>) -> bool,
) {
    while !p.at(EOF) && !is_end(p) {
        if p.at(delim) {
            // Recover if an argument is missing and only got a delimiter,
            // e.g. `(a, , b)`.
            // Wrap the erroneous delimiter in an error node so that fixup logic gets rid of it.
            let m = p.start();
            p.error(unexpected_delim_message());
            p.bump(delim);
            m.complete(p, ERROR);
            continue;
        }
        if !parser(p) {
            break;
        }
        if !p.eat(delim) {
            if at_item_first(p) {
                p.error(format!("expected {delim:?}"));
            } else {
                break;
            }
        }
    }
}
