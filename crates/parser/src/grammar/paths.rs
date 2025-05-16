use crate::grammar::type_args::opt_path_type_arg_list;
use crate::grammar::{any_address, items, name_ref};
use crate::parser::{CompletedMarker, Parser};
use crate::token_set::TokenSet;
use crate::SyntaxKind::*;
use crate::T;

pub(super) const PATH_FIRST: TokenSet = TokenSet::new(&[IDENT, INT_NUMBER]);

pub(super) fn is_path_start(p: &Parser) -> bool {
    match p.current() {
        // addresses
        INT_NUMBER if p.nth_at(1, T![::]) => true,
        IDENT /*| T![self] | T![super] | T![crate]*/ => true,
        T![::] => true,
        T!['_'] => true,
        // T![:] if p.at(T![::]) => true,
        _ => false,
    }
}

pub(super) fn use_path(p: &mut Parser) {
    path(p, Mode::Use);
}

pub(crate) fn type_path(p: &mut Parser) {
    path(p, Mode::Type);
}

pub(super) fn expr_path(p: &mut Parser) {
    path(p, Mode::Expr);
}

pub(crate) fn type_path_for_qualifier(p: &mut Parser, qual: CompletedMarker) -> CompletedMarker {
    path_for_qualifier(p, Mode::Type, qual)
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum Mode {
    Use,
    Type,
    Expr,
}

fn path(p: &mut Parser, mode: Mode) {
    let path = p.start();
    path_segment(p, mode, true);
    let qual = path.complete(p, PATH);
    path_for_qualifier(p, mode, qual);
}

fn path_for_qualifier(p: &mut Parser, mode: Mode, mut qual: CompletedMarker) -> CompletedMarker {
    loop {
        let use_tree = matches!(p.nth(1), T!['{']);
        if p.at(T![::]) && !use_tree {
            let path = qual.precede(p);
            p.bump(T![::]);
            path_segment(p, mode, false);
            let path = path.complete(p, PATH);
            qual = path;
        } else {
            return qual;
        }
    }
}

fn path_segment(p: &mut Parser, mode: Mode, first: bool) {
    let m = p.start();

    let empty = if first { !p.eat(T![::]) } else { true };
    match p.current() {
        T!['_'] => {
            let m = p.start();
            p.bump_remap(IDENT);
            m.complete(p, NAME_REF);
        }
        IDENT => {
            name_ref(p);
            opt_path_type_arg_list(p, mode);
        }
        INT_NUMBER if first => {
            let m = p.start();
            any_address(p);
            m.complete(p, PATH_ADDRESS);
        }
        _ => {
            p.error_and_bump_until("expected identifier", items::item_start);
            if empty {
                // test_err empty_segment
                // use crate::;
                m.abandon(p);
                return;
            }
        }
    };

    m.complete(p, PATH_SEGMENT);
}
