use crate::parse::parser::Parser;
use crate::parse::token_set::TokenSet;
use crate::SyntaxKind::{EOF, ERROR};
use crate::{SyntaxKind, T};

pub(crate) fn delimited_with_recovery(
    p: &mut Parser,
    element: impl Fn(&mut Parser) -> bool,
    delimiter: SyntaxKind,
    expected_element_error: &str,
    list_end: Option<SyntaxKind>,
) {
    let mut iteration = 0;

    let outer_recovery_set = p.outer_recovery_set();
    let list_end_ts = list_end.map(|it| it.into()).unwrap_or(TokenSet::EMPTY);

    // cannot recover if delimiter divides outer elements
    let modified_recovery_set = outer_recovery_set
        .clone()
        .with_token_set(list_end_ts)
        .without_recovery_token(delimiter.into());
    let at_list_end = |p: &Parser| p.at_ts(list_end_ts);

    let outer_recovery_on_delimiter = outer_recovery_set.contains(delimiter);

    let mut is_empty = true;
    while !p.at(EOF) && !at_list_end(p) {
        #[cfg(debug_assertions)]
        let _p = stdx::panic_context::enter(format!("p.text_context() = {:?}", p.current_context()));

        // check whether we can parse element, if not, then recover till the delimiter / end of the list
        let mut recover_set = TokenSet::new(&[delimiter]);
        if let Some(list_end) = list_end {
            recover_set = recover_set | list_end;
        }
        let at_element = p.with_recovery_token_set(recover_set, |p| element(p));
        if at_element {
            is_empty = false;
        }
        if !at_element {
            // if outer recovery set has delimiter, we can't recover in inner lists
            if outer_recovery_on_delimiter {
                break;
            }
            // if list is empty
            if list_end.is_some() && is_empty && at_list_end(p) {
                break;
            }
            p.error_and_recover(expected_element_error, delimiter);
        }

        if modified_recovery_set.contains_current(p) {
            break;
        }
        let is_delim = p.expect(delimiter);
        if is_delim {
            if delimiter == T![,] && modified_recovery_set.contains_current(p) {
                // trailing comma
                break;
            }
        }

        iteration += 1;
        if iteration > 1000 {
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
