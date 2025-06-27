pub(crate) mod predicates;
pub(crate) mod quants;
pub(crate) mod schemas;

use crate::parse::grammar::expressions::atom::block_expr;
use crate::parse::parser::{CompletedMarker, Parser};
use crate::SyntaxKind::SPEC_BLOCK_EXPR;
use crate::T;

pub(crate) fn opt_spec_block_expr(p: &mut Parser) {
    if p.at(T![spec]) {
        spec_block_expr(p);
    }
}

pub(crate) fn spec_block_expr(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T![spec]));
    let m = p.start();
    p.bump(T![spec]);
    if p.at(T!['{']) {
        block_expr(p, true);
    } else {
        p.error("expected a block");
    }
    m.complete(p, SPEC_BLOCK_EXPR)
}
