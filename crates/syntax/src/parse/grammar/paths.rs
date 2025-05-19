use crate::parse::grammar::type_args::opt_path_type_arg_list;
use crate::parse::grammar::{any_address, items, name_ref, value_address};
use crate::parse::parser::{CompletedMarker, Parser};
use crate::parse::token_set::TokenSet;
use crate::SyntaxKind::*;
use crate::{ts, T};

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
    path(p, Mode::Use, ts!());
}

pub(crate) fn type_path(p: &mut Parser) {
    path(p, Mode::Type, ts!());
}

pub(super) fn expr_path(p: &mut Parser) {
    path(p, Mode::Expr, ts!());
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

pub(crate) fn path(p: &mut Parser, mode: Mode, additional_recovery_set: TokenSet) {
    let path = p.start();
    path_segment(p, mode, true, additional_recovery_set);
    let qual = path.complete(p, PATH);
    path_for_qualifier(p, mode, qual);
}

fn path_for_qualifier(p: &mut Parser, mode: Mode, mut qual: CompletedMarker) -> CompletedMarker {
    loop {
        let use_tree = matches!(p.nth(1), T!['{']);
        if p.at(T![::]) && !use_tree {
            let path = qual.precede(p);
            p.bump(T![::]);
            path_segment(p, mode, false, ts!());
            let path = path.complete(p, PATH);
            qual = path;
        } else {
            return qual;
        }
    }
}

fn path_segment(p: &mut Parser, mode: Mode, first: bool, additional_recovery_set: TokenSet) {
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
            value_address(p);
            m.complete(p, PATH_ADDRESS);
        }
        _ => {
            p.error_and_bump_until("expected identifier", |p| {
                items::at_item_start(p) || p.at_ts(additional_recovery_set)
            });
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
