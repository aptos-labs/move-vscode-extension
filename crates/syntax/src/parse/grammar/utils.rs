use crate::parse::parser::Parser;
use crate::parse::token_set::TokenSet;
use crate::SyntaxKind::{EOF, ERROR};
use crate::{ts, SyntaxKind, T};

pub(crate) fn delimited_items_with_recover(
    p: &mut Parser,
    rbrace: SyntaxKind,
    delim: SyntaxKind,
    end_at: TokenSet,
    item_kind: SyntaxKind,
    mut parse_item: impl FnMut(&mut Parser) -> bool,
) {
    while !p.at(EOF) && !p.at(rbrace) && !p.at_ts(end_at) {
        if p.at(delim) {
            // Recover if an argument is missing and only got a delimiter,
            // e.g. `(a, , b)`.
            let empty_item = p.start();
            p.push_error(format!("unexpected {:?}", delim));
            empty_item.complete(p, item_kind);
            p.bump(delim);
            continue;
        }
        let is_item = parse_item(p);
        if !is_item {
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
    p: &mut Parser,
    bra: SyntaxKind,
    ket: SyntaxKind,
    delim: SyntaxKind,
    unexpected_delim_message: impl Fn() -> String,
    item_first_set: TokenSet,
    parser: impl FnMut(&mut Parser) -> bool,
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
//     parser: impl FnMut(&mut Parser) -> bool,
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
    parse: impl FnMut(&mut Parser) -> bool,
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

// type ParserAt = dyn Fn(&Parser) -> bool;

pub(crate) fn delimited_fn(
    p: &mut Parser,
    delim: SyntaxKind,
    unexpected_delim_message: impl Fn() -> String,
    is_end: impl Fn(&Parser) -> bool,
    at_item_first: impl Fn(&Parser) -> bool,
    mut parse: impl FnMut(&mut Parser) -> bool,
) {
    let mut iteration = 0;
    while !p.at(EOF) && !is_end(p) {
        iteration += 1;
        if iteration > 1000 {
            // something's wrong and we don't want to hang
            break;
        }
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

pub(crate) fn delimited_with_recovery(
    p: &mut Parser,
    at_list_end: SyntaxKind,
    // at_element_first: TokenSet,
    element: impl Fn(&mut Parser) -> bool,
    delimiter: SyntaxKind,
    expected_element: &str,
    element_recovery_set: TokenSet,
) {
    delimited_with_recovery_fn(
        p,
        |p| p.at_ts(ts!(at_list_end)),
        // |p| p.at_ts(at_element_first),
        element,
        delimiter,
        expected_element,
        element_recovery_set,
    )
}

pub(crate) fn delimited_with_recovery_fn(
    p: &mut Parser,
    at_list_end: impl Fn(&Parser) -> bool,
    // at_element_first: impl Fn(&Parser) -> bool,
    element: impl Fn(&mut Parser) -> bool,
    delimiter: SyntaxKind,
    expected_element_error: &str,
    element_recovery_set: TokenSet,
) {
    let mut iteration = 0;
    let outer_recovery_set = p.outer_recovery_set();

    // cannot recover if there delimiter divides outer elements
    let should_not_recover = outer_recovery_set.contains(delimiter);
    let modified_recovery_set = outer_recovery_set.sub(ts!(delimiter));

    // let at_list_end = |p: &Parser| p.at_ts(outer_recovery_set) || at_list_end(p);
    while !p.at(EOF) && !p.at_ts(modified_recovery_set) && !at_list_end(p) {
        #[cfg(debug_assertions)]
        let _p = stdx::panic_context::enter(format!("p.text_context() = {:?}", p.text_context(),));

        // check whether we can parse element, if not, then recover till the delimiter / end of the list
        let at_element = element(p);
        if !at_element {
            if should_not_recover {
                // should stop here
                break;
            }
            p.error_and_recover_until_ts(expected_element_error, element_recovery_set + ts!(delimiter));
        }

        if p.at_ts(modified_recovery_set) {
            break;
        }

        // if at_element_first(p) {
        //     element(p);
        // } else {
        //     p.error_and_recover_until_ts(expected_element, element_recovery_set);
        // }
        if !at_list_end(p) {
            p.expect(delimiter);
        }

        iteration += 1;
        if iteration > 100 {
            // something's wrong and we don't want to hang
            #[cfg(debug_assertions)]
            {
                panic!(
                    "at {:?}: reached limit iteration in delimited_with_recovery_fn() loop, at_element = {at_element}",
                    p.current()
                );
            }
            break;
        }
    }
}
