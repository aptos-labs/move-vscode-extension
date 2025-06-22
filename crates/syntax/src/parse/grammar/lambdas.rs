use crate::parse::grammar::patterns::PAT_RECOVERY_SET;
use crate::parse::grammar::utils::delimited_with_recovery;
use crate::parse::grammar::{patterns, types};
use crate::parse::parser::Parser;
use crate::parse::token_set::TokenSet;
use crate::SyntaxKind::*;
use crate::{ts, T};

pub(crate) fn lambda_param_list(p: &mut Parser) -> bool {
    let list_marker = p.start();
    p.bump(T![|]);
    if p.at(T![,]) {
        list_marker.abandon(p);
        return false;
    }

    delimited_with_recovery(p, lambda_param, T![,], "expected ident", Some(T![|]));
    // delimited_items_with_recover(p, T![|], T![,], ts!(), LAMBDA_PARAM, lambda_param);

    if !p.eat(T![|]) {
        list_marker.abandon_with_rollback(p);
        return false;
    }

    list_marker.complete(p, LAMBDA_PARAM_LIST);
    true
}

fn lambda_param(p: &mut Parser) -> bool {
    let m = p.start();
    let is_ident = patterns::ident_or_wildcard_pat_with_recovery(p);
    if is_ident {
        if p.at(T![:]) {
            types::ascription(p);
        }
    }
    m.complete(p, LAMBDA_PARAM);
    true
}
