use crate::grammar::utils::list;
use crate::grammar::{address, address_ref, generic_args, items, name_ref, types};
use crate::parser::{CompletedMarker, Parser};
use crate::token_set::TokenSet;
use crate::SyntaxKind::*;
use crate::T;

pub(super) const PATH_FIRST: TokenSet = TokenSet::new(&[IDENT, INT_NUMBER]);

pub(super) fn is_path_start(p: &Parser) -> bool {
    is_use_path_start(p)
}

pub(super) fn is_use_path_start(p: &Parser) -> bool {
    match p.current() {
        // addresses
        INT_NUMBER if p.nth_at(1, T![::]) => true,
        IDENT /*| T![self] | T![super] | T![crate]*/ => true,
        T![::] => true,
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

    let mut empty = if first { !p.eat(T![::]) } else { true };
    // if p.at(IDENT) {
    //     name_ref(p);
    //     opt_path_args(p, mode);
    // }
    match p.current() {
        INT_NUMBER if first => {
            let m = p.start();
            address(p);
            m.complete(p, PATH_ADDRESS);
        }
        IDENT => {
            name_ref(p);
            opt_path_args(p, mode);
            // opt_path_type_args(p, mode);
        }
        _ => {
            p.err_recover("expected identifier", items::ITEM_KW_RECOVERY_SET);
            if empty {
                // test_err empty_segment
                // use crate::;
                m.abandon(p);
                return;
            }
        }
    };

    // // test qual_paths
    // // type X = <A as B>::Output;
    // // fn foo() { <usize as Default>::default(); }
    //
    //
    // if first && p.eat(T![<]) {
    //     types::type_(p);
    //     if p.eat(T![as]) {
    //         if is_use_path_start(p) {
    //             types::path_type(p);
    //         } else {
    //             p.error("expected a trait");
    //         }
    //     }
    //     p.expect(T![>]);
    // } else {
    // }
    m.complete(p, PATH_SEGMENT);
}

pub(crate) fn opt_path_type_args(p: &mut Parser<'_>) {
    // test typepathfn_with_coloncolon
    // type F = Start::(Middle) -> (Middle)::End;
    // type GenericArg = S<Start(Middle)::End>;
    // let m;
    // if p.at(T![::]) && matches!(p.nth(2), T![<] | T!['(']) {
    //     m = p.start();
    //     p.bump(T![::]);
    // } else if (p.current() == T![<] && p.nth(1) != T![=]) || p.current() == T!['('] {
    //     m = p.start();
    // } else {
    //     return;
    // }
    let m = p.start();
    let current = p.current();
    if current != T![<] {
        m.abandon(p);
        return;
    }
    // test_err generic_arg_list_recover
    // type T = T<0, ,T>;
    // type T = T::<0, ,T>;
    list(
        p,
        T![<],
        T![>],
        T![,],
        || "expected generic argument".into(),
        generic_args::GENERIC_ARG_FIRST,
        generic_args::generic_arg,
    );
    m.complete(p, TYPE_ARG_LIST);
}

pub(crate) fn opt_path_args(p: &mut Parser<'_>, mode: Mode) {
    match mode {
        Mode::Use /*| Mode::Attr | Mode::Vis*/ => {}
        Mode::Type => opt_path_type_args(p),
        Mode::Expr => generic_args::opt_generic_arg_list_expr(p, false),
    }
}
