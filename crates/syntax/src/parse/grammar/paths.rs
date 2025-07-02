use crate::SyntaxKind::*;
use crate::parse::grammar::items::item_start_rec_set;
use crate::parse::grammar::type_args::opt_path_type_arg_list;
use crate::parse::grammar::{any_address, items, name_ref, value_address};
use crate::parse::parser::{CompletedMarker, Parser};
use crate::parse::token_set::TokenSet;
use crate::{T, ts};

pub(super) const PATH_FIRST: TokenSet = TokenSet::new(&[IDENT, INT_NUMBER]);

pub(super) fn is_path_start(p: &Parser) -> bool {
    match p.current() {
        // addresses
        INT_NUMBER if p.nth_at(1, T![::]) => true,
        IDENT => true,
        T![::] => true,
        T!['_'] => true,
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum Mode {
    Use,
    Type,
    Expr,
}

pub(crate) fn path(p: &mut Parser, mode: Mode) {
    let path = p.start();
    path_segment(p, mode, true);
    let qual = path.complete(p, PATH);
    path_for_qualifier(p, mode, qual);
}

fn path_for_qualifier(p: &mut Parser, mode: Mode, mut qual: CompletedMarker) -> CompletedMarker {
    loop {
        let is_use_tree = matches!(p.nth(1), T!['{']);
        if p.at(T![::]) && !is_use_tree {
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
            #[cfg(debug_assertions)]
            let _p = stdx::panic_context::enter(format!("path_segment {:?}", p.current_text()));
            name_ref(p);
            opt_path_type_arg_list(p, mode);
        }
        INT_NUMBER if first => {
            let m = p.start();
            value_address(p);
            m.complete(p, PATH_ADDRESS);
        }
        _ => {
            p.error_and_recover("expected identifier", item_start_rec_set());
            if empty {
                m.abandon(p);
                return;
            }
        }
    };

    m.complete(p, PATH_SEGMENT);
}
