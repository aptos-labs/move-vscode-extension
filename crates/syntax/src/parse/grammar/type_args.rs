use super::*;
use crate::parse::grammar::paths::Mode;
use crate::parse::grammar::types::TYPE_FIRST;
use crate::parse::grammar::utils::{
    delimited, delimited_items_with_recover, delimited_with_recovery, list,
};
use stdx::panic_context::PanicContext;

pub(crate) fn opt_path_type_arg_list(p: &mut Parser<'_>, mode: Mode) {
    match mode {
        Mode::Use /*| Mode::Attr */ => {}
        Mode::Type => opt_type_arg_list_for_type(p),
        Mode::Expr => opt_type_arg_list_for_expr(p, false),
    }
}

pub(crate) fn opt_type_arg_list_for_type(p: &mut Parser<'_>) {
    let _p = stdx::panic_context::enter("opt_type_arg_list_for_type".to_string());
    let m = p.start();
    let current = p.current();
    if current != T![<] {
        m.abandon(p);
        return;
    }
    p.bump(T![<]);
    delimited_with_recovery(
        p,
        T![>],
        // TYPE_ARG_FIRST + TYPE_FIRST,
        |p| type_arg(p),
        T![,],
        "expected generic argument",
        TokenSet(!0), // no recovery
    );
    // delimited(
    //     p,
    //     T![,],
    //     || "expected generic argument".into(),
    //     |p| p.at(T![>]),
    //     TYPE_ARG_FIRST,
    //     type_arg,
    // );
    p.expect(T![>]);
    m.complete(p, TYPE_ARG_LIST);
}

pub(super) fn opt_type_arg_list_for_expr(p: &mut Parser<'_>, colon_colon_required: bool) {
    let m;
    if p.at(T![::]) && p.nth(1) == T![<] {
        m = p.start();
        p.bump(T![::]);
    } else if !colon_colon_required && p.at(T![<]) {
        m = p.start();
    } else {
        return;
    }
    p.bump(T![<]);

    let at_end = |p: &Parser| p.at_ts(ts!(T![>], T!['('], T!['{']));
    while !p.at(EOF) && !at_end(p) {
        if !type_arg(p) {
            break;
        }
        if !p.eat(T![,]) {
            if p.at_ts(TYPE_ARG_FIRST) {
                p.error("expected ','");
            } else {
                break;
            }
        }
    }
    if !p.eat(T![>]) {
        m.abandon_with_rollback(p);
        return;
    }
    m.complete(p, TYPE_ARG_LIST);
}

pub(crate) const TYPE_ARG_FIRST: TokenSet = TokenSet::new(&[IDENT]);
// .union(types::TYPE_FIRST);

// Despite its name, it can also be used for generic param list.
const GENERIC_ARG_RECOVERY_SET: TokenSet = TokenSet::new(&[T![>], T![,]]);

// test generic_arg
// type T = S<i32, dyn T, fn()>;
pub(crate) fn type_arg(p: &mut Parser<'_>) -> bool {
    match p.current() {
        IDENT => {
            let m = p.start();
            name_ref(p);
            opt_path_type_arg_list(p, Mode::Type);

            let m = m.complete(p, PATH_SEGMENT).precede(p).complete(p, PATH);
            let m = paths::type_path_for_qualifier(p, m);

            let m = m.precede(p).complete(p, PATH_TYPE);
            m.precede(p).complete(p, TYPE_ARG);
        }
        _ if p.at_ts(TYPE_FIRST) => {
            let m = p.start();
            types::type_(p);
            m.complete(p, TYPE_ARG);
        }
        _ => return false,
    }
    true
}
