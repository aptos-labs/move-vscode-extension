pub(crate) mod predicates;
pub(crate) mod quants;
pub(crate) mod schemas;

use crate::grammar::expressions::atom::block_expr;
use crate::parser::CompletedMarker;
use crate::SyntaxKind::SPEC_BLOCK_EXPR;
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
