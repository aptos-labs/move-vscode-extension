use super::*;
use crate::parse::grammar::paths::Mode;
use crate::parse::grammar::types::{TYPE_FIRST, TYPE_FIRST_NO_LAMBDA};
use crate::parse::grammar::utils::delimited_with_recovery;
use crate::TextSize;
use std::ops::ControlFlow::{Break, Continue};

pub(crate) fn opt_path_type_arg_list(p: &mut Parser, mode: Mode) {
    match mode {
        Mode::Use => {}
        Mode::Type => opt_type_arg_list_for_type(p),
        Mode::Expr => opt_type_arg_list_for_expr(p, false),
    }
}

pub(crate) fn opt_type_arg_list_for_type(p: &mut Parser) {
    let _p = stdx::panic_context::enter("opt_type_arg_list_for_type".to_string());
    if !p.at(T![<]) {
        return;
    }
    let m = p.start();
    p.bump(T![<]);
    p.with_recovery_token(T![>], |p| {
        delimited_with_recovery(
            p,
            |p| type_arg(p, true),
            T![,],
            "expected type argument",
            Some(T![>]),
        )
    });
    p.expect(T![>]);
    m.complete(p, TYPE_ARG_LIST);
}

pub(super) fn opt_type_arg_list_for_expr(p: &mut Parser, colon_colon_required: bool) {
    let m;
    if p.at(T![::]) && p.nth_at(1, T![<]) {
        m = p.start();
        p.bump(T![::]);
    } else {
        if !p.at(T![<]) {
            return;
        }
        // '::' is optional if there's no whitespace between ident and '<'
        if !colon_colon_required || p.prev_ws_at(0) == 0 {
            m = p.start();
        } else {
            return;
        }
    }

    p.bump(T![<]);

    // NOTE: we cannot add recovery in type args for expr, it's ambiguous with the lt/gt expr
    let mut has_error = false;
    p.iterate_to_EOF(T![>] | T!['('] | T!['{'], |p| {
        if !type_arg(p, false) {
            has_error = true;
            return Break(());
        }
        if !p.eat(T![,]) {
            if p.at_ts(TYPE_ARG_FIRST) {
                p.error("expected ','");
                has_error = true;
            } else {
                return Break(());
            }
        }
        Continue(())
    });
    if has_error || !p.eat(T![>]) {
        m.abandon_with_rollback(p);
        return;
    }
    m.complete(p, TYPE_ARG_LIST);
}

pub(crate) const TYPE_ARG_FIRST: TokenSet = TokenSet::new(&[IDENT]);
// .union(types::TYPE_FIRST);

pub(crate) fn type_arg(p: &mut Parser, is_type: bool) -> bool {
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
            let mut rec = vec![T![,]];
            let mut rec_token_set = TokenSet::from(T![,]);
            // can't recover at T![>] in expr due to ambiguity
            if is_type {
                rec_token_set = rec_token_set | T![>];
            }
            let is_valid_type = p.with_recovery_token_set(rec_token_set, types::type_);
            if !is_type && !is_valid_type {
                // have to be safe
                m.abandon(p);
                return false;
            }
            m.complete(p, TYPE_ARG);
        }
        _ => return false,
    }
    true
}
