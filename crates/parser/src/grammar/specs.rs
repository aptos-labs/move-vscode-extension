pub(crate) mod predicates;
pub(crate) mod quants;
pub(crate) mod schemas;

use crate::grammar::expressions::atom::block_expr;
use crate::grammar::expressions::expr;
use crate::parser::CompletedMarker;
use crate::SyntaxKind::{IDENT, SPEC_BLOCK_EXPR, SPEC_PREDICATE_STMT};
use crate::{Parser, T};

pub(crate) fn opt_spec_block_expr(p: &mut Parser) {
    if p.at(T![spec]) {
        spec_block_expr(p);
    }
}

pub(crate) fn spec_block_expr(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T![spec]));
    let m = p.start();
    p.bump(T![spec]);
    block_expr(p, true);
    m.complete(p, SPEC_BLOCK_EXPR)
}

pub(crate) static PREFIX_PREDICATES: &[&str] = &[
    "assume",
    "assert",
    "requires",
    "ensures",
    "aborts_if",
    "aborts_with",
    "decreases",
];
