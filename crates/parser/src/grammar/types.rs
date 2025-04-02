use crate::grammar::paths;
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
    type_with_bounds_cond(p, true)
}

pub(super) fn type_no_bounds(p: &mut Parser) {
    type_with_bounds_cond(p, false);
}

fn type_with_bounds_cond(p: &mut Parser, allow_bounds: bool) {
    match p.current() {
        T!['('] => paren_or_tuple_or_unit_type(p),
        T![&] => ref_type(p),
        T![|] => lambda_type(p),
        // T![<] => path_type_(p, allow_bounds),
        _ if paths::is_use_path_start(p) => path_or_macro_type_(p, allow_bounds),
        _ => {
            p.error_and_recover_until_ts("expected type", TYPE_RECOVERY_SET);
        }
    }
}

// test path_type
// type A = Foo;
// type B = ::Foo;
// type C = self::Foo;
// type D = super::Foo;
pub(super) fn path_type(p: &mut Parser) {
    path_type_(p, true);
}

// test macro_call_type
// type A = foo!();
// type B = crate::foo!();
fn path_or_macro_type_(p: &mut Parser, allow_bounds: bool) {
    assert!(paths::is_path_start(p));
    let r = p.start();
    let m = p.start();

    paths::type_path(p);

    let kind = /*if p.at(T![!]) && !p.at(T![!=]) {
        items::macro_call_after_excl(p);
        m.complete(p, MACRO_CALL);
        MACRO_TYPE
    } else*/ {
        m.abandon(p);
        PATH_TYPE
    };

    let path = r.complete(p, kind);

    // if allow_bounds {
    //     opt_type_bounds_as_dyn_trait_type(p, path);
    // }
}

pub(super) fn path_type_(p: &mut Parser, allow_bounds: bool) {
    assert!(paths::is_path_start(p));
    let m = p.start();
    paths::type_path(p);

    // test path_type_with_bounds
    // fn foo() -> Box<T + 'f> {}
    // fn foo() -> Box<dyn T + 'f> {}
    let path = m.complete(p, PATH_TYPE);
    // if allow_bounds {
    //     opt_type_bounds_as_dyn_trait_type(p, path);
    // }
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
    type_no_bounds(p);
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
    m.complete(p, LAMBDA_TYPE);
}

fn unit_type(p: &mut Parser) {
    assert!(p.at(T!['(']));
    let m = p.start();
    p.bump(T!['(']);
    p.bump(T![')']);
    m.complete(p, UNIT_TYPE);
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
