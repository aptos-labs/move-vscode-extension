use crate::grammar::utils::{list_with_recover, list_with_recover_inner};
use crate::grammar::{patterns, types};
use crate::SyntaxKind::*;
use crate::{ts, Parser, T};

pub(crate) fn lambda_param_list(p: &mut Parser) -> bool {
    let list_marker = p.start();
    p.bump(T![|]);
    if p.at(T![,]) {
        list_marker.abandon(p);
        return false;
    }

    list_with_recover_inner(p, T![|], T![,], ts!(), lambda_param);

    // delimited(
    //     p,
    //     T![,],
    //     || "expected parameter".into(),
    //     |p| p.at(T![|]),
    //     ts!(IDENT, T!['_']),
    //     |p| {
    //         let m = p.start();
    //         patterns::pattern(p);
    //         if p.at(T![:]) {
    //             types::ascription(p);
    //         }
    //         m.complete(p, LAMBDA_PARAM);
    //         true
    //     },
    // );
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
