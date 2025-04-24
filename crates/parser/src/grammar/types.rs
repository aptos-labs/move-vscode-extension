use crate::grammar::generic_params::ability_bound_list;
use crate::grammar::{ability, paths};
use crate::grammar::utils::delimited;
use crate::parser::Parser;
use crate::token_set::TokenSet;
use crate::SyntaxKind::*;
use crate::T;

pub(super) const TYPE_FIRST: TokenSet =
    paths::PATH_FIRST.union(TokenSet::new(&[T!['('], T!['['], T![<], T![!], T![*], T![&]]));

pub(super) const TYPE_RECOVERY_SET: TokenSet = TokenSet::new(&[
    T![')'],
    T![>],
    T![,],
    // test_err struct_field_recover
    // struct S { f pub g: () }
    // T![pub],
]);

pub(super) fn ascription(p: &mut Parser) {
    assert!(p.at(T![:]));
    p.bump(T![:]);
    type_(p);
}

pub(crate) fn type_(p: &mut Parser) {
    type_or_recover_until(p, |_| true)
    // match p.current() {
    //     T!['('] => paren_or_tuple_or_unit_type(p),
    //     T![&] => ref_type(p),
    //     T![|] => lambda_type(p),
    //     _ if paths::is_path_start(p) => path_type(p),
    //     _ => {
    //         p.error("expected type");
    //         // p.error_and_recover_until_ts("expected type", TYPE_RECOVERY_SET);
    //     }
    // }
}

pub(crate) fn type_or_recover_until(p: &mut Parser, stop: impl Fn(&Parser) -> bool) {
    match p.current() {
        T!['('] => paren_or_tuple_or_unit_type(p),
        T![&] => ref_type(p),
        T![|] => lambda_type(p),
        _ if paths::is_path_start(p) => path_type(p),
        _ => {
            // p.error("expected type");
            // return false
            p.error_and_bump_until("expected type", stop);
            // p.error("expected type");
            // p.error_and_recover_until_ts("expected type", TYPE_RECOVERY_SET);
        }
    }
    // true
}

pub(super) fn path_type(p: &mut Parser) {
    assert!(paths::is_path_start(p));

    let m = p.start();
    paths::type_path(p);

    m.complete(p, PATH_TYPE);
}

// test reference_type;
// type A = &();
// type B = &'static ();
// type C = &mut ();
fn ref_type(p: &mut Parser) {
    assert!(p.at(T![&]));
    let m = p.start();
    p.bump(T![&]);
    p.eat(T![mut]);
    type_(p);
    m.complete(p, REF_TYPE);
}

fn lambda_type(p: &mut Parser) {
    assert!(p.at(T![|]));
    let m = p.start();
    p.bump(T![|]);
    if p.at(T![,]) {
        m.abandon_with_rollback(p);
        return;
    }
    delimited(
        p,
        T![,],
        || "unexpected type".into(),
        |p| p.at(T![|]),
        TYPE_FIRST,
        |p| {
            let m = p.start();
            type_(p);
            m.complete(p, LAMBDA_TYPE_PARAM);
            true
        },
    );
    if !p.eat(T![|]) {
        m.abandon_with_rollback(p);
        return;
    }
    if p.at_ts(TYPE_FIRST) {
        type_(p);
    }
    if p.at_contextual_kw_ident("has") {
        let m = p.start();
        p.bump_remap(T![has]);
        ability_bound_list(p);
        m.complete(p, LAMBDA_TYPE_ABILITY_LIST);
    }
    m.complete(p, LAMBDA_TYPE);
}

fn paren_or_tuple_or_unit_type(p: &mut Parser) {
    assert!(p.at(T!['(']));
    let m = p.start();
    p.bump(T!['(']);
    let mut n_types: u32 = 0;
    let mut trailing_comma: bool = false;
    while !p.at(EOF) && !p.at(T![')']) {
        n_types += 1;
        type_(p);
        if p.eat(T![,]) {
            trailing_comma = true;
        } else {
            trailing_comma = false;
            break;
        }
    }
    p.expect(T![')']);

    let kind = if n_types == 1 && !trailing_comma {
        // test paren_type
        // type T = (i32);
        PAREN_TYPE
    } else if n_types == 0 {
        UNIT_TYPE
    } else {
        // test unit_type
        // type T = ();

        // test singleton_tuple_type
        // type T = (i32,);
        TUPLE_TYPE
    };
    m.complete(p, kind);
}
