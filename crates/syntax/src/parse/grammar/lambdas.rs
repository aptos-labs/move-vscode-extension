use crate::parse::grammar::utils::delimited_items_with_recover;
use crate::parse::grammar::{patterns, types};
use crate::parse::parser::Parser;
use crate::SyntaxKind::*;
use crate::{ts, T};

pub(crate) fn lambda_param_list(p: &mut Parser) -> bool {
    let list_marker = p.start();
    p.bump(T![|]);
    if p.at(T![,]) {
        list_marker.abandon(p);
        return false;
    }

    delimited_items_with_recover(p, T![|], T![,], ts!(), LAMBDA_PARAM, lambda_param);

    if !p.eat(T![|]) {
        list_marker.abandon_with_rollback(p);
        return false;
    }

    list_marker.complete(p, LAMBDA_PARAM_LIST);
    true
}

fn lambda_param(p: &mut Parser<'_>) -> bool {
    let m = p.start();
    let completed = patterns::pattern(p);
    match completed.map(|it| it.kind()) {
        Some(IDENT_PAT) | Some(WILDCARD_PAT) => (),
        _ => {
            p.push_error("expected ident or wildcard pattern");
            // false
        }
    }
    // if !is_completed {
    //     m.abandon(p);
    //     return false;
    // }
    if p.at(T![:]) {
        types::ascription(p);
    }
    m.complete(p, LAMBDA_PARAM);
    true
}
