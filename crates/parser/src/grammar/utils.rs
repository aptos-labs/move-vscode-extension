use crate::token_set::TokenSet;
use crate::SyntaxKind::{EOF, ERROR};
use crate::{Parser, SyntaxKind, T};

pub(crate) fn list_with_recover(
    p: &mut Parser<'_>,
    lbrace: SyntaxKind,
    rbrace: SyntaxKind,
    delim: SyntaxKind,
    _unexpected_delim_message: impl Fn() -> String,
    end_at: TokenSet,
    _item_first: TokenSet,
    parse_item: impl FnMut(&mut Parser<'_>) -> bool,
) {
    p.bump(lbrace);

    // let at_item_first = |p: &mut Parser<'_>| p.at_ts(item_first);

    list_with_recover_inner(p, rbrace, delim, end_at, parse_item);

    // while !p.at(EOF) && !p.at_ts(end_at) {
    //     if p.at(delim) {
    //         // Recover if an argument is missing and only got a delimiter,
    //         // e.g. `(a, , b)`.
    //         // Wrap the erroneous delimiter in an error node so that fixup logic gets rid of it.
    //         let m = p.start();
    //         p.error(unexpected_delim_message());
    //         p.bump(delim);
    //         m.complete(p, ERROR);
    //         continue;
    //     }
    //     if !parse_item(p) {
    //         break;
    //     }
    //     if !p.eat(delim) {
    //         if at_item_first(p) {
    //             p.error(format!("expected {delim:?}"));
    //         } else {
    //             break;
    //         }
    //     }
    // }

    // delimited(
    //     p,
    //     delim,
    //     unexpected_delim_message,
    //     |p| p.at(rbrace) || p.at_ts(end_at),
    //     item_first,
    //     parse_item,
    // );
    p.expect(rbrace);
}

pub(crate) fn list_with_recover_inner(
    p: &mut Parser<'_>,
    rbrace: SyntaxKind,
    delim: SyntaxKind,
    end_at: TokenSet,
    mut parse_item: impl FnMut(&mut Parser<'_>) -> bool,
) {
    while !p.at(EOF) && !p.at(rbrace) && !p.at_ts(end_at) {
        if p.at(delim) {
            // Recover if an argument is missing and only got a delimiter,
            // e.g. `(a, , b)`.
            // Wrap the erroneous delimiter in an error node so that fixup logic gets rid of it.
            let m = p.start();
            p.error(format!("unexpected {:?}", delim));
            p.bump(delim);
            m.complete(p, ERROR);
            continue;
        }
        let is_item = parse_item(p);
        if !is_item {
            // p.bump_until(|p| p.at(delim) || p.at_ts(end_at) || p.at(rbrace) || p.at(EOF));
            // invalid item encountered, stop iterating
            break;
        }
        if !p.eat(delim) {
            break;
        }
    }
}

/// The `parser` passed this is required to at least consume one token if it returns `true`.
/// If the `parser` returns false, parsing will stop.
pub(crate) fn list(
    p: &mut Parser<'_>,
    bra: SyntaxKind,
    ket: SyntaxKind,
    delim: SyntaxKind,
    unexpected_delim_message: impl Fn() -> String,
    item_first_set: TokenSet,
    parser: impl FnMut(&mut Parser<'_>) -> bool,
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

// pub(crate) fn comma_separated_list(
//     p: &mut Parser,
//     unexpected_delim_message: &str,
//     is_end: impl Fn(&Parser) -> bool,
//     item_first_set: TokenSet,
//     parser: impl FnMut(&mut Parser<'_>) -> bool,
// ) {
//     delimited(
//         p,
//         T![,],
//         || unexpected_delim_message.into(),
//         is_end,
//         item_first_set,
//         parser,
//     )
// }

pub(crate) fn delimited(
    p: &mut Parser,
    delim: SyntaxKind,
    unexpected_delim_message: impl Fn() -> String,
    is_end: impl Fn(&Parser) -> bool,
    item_first_set: TokenSet,
    parse: impl FnMut(&mut Parser<'_>) -> bool,
) {
    delimited_fn(
        p,
        delim,
        unexpected_delim_message,
        is_end,
        |p| p.at_ts(item_first_set),
        parse,
    );
}

pub(crate) fn delimited_fn(
    p: &mut Parser,
    delim: SyntaxKind,
    unexpected_delim_message: impl Fn() -> String,
    is_end: impl Fn(&Parser) -> bool,
    at_item_first: impl Fn(&Parser) -> bool,
    mut parse: impl FnMut(&mut Parser<'_>) -> bool,
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
        if !parse(p) {
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
