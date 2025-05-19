use crate::parse::grammar::{error_block, expressions, name, name_ref, paths};
use crate::parse::parser::{CompletedMarker, Parser};
use crate::parse::token_set::TokenSet;
use crate::SyntaxKind::*;
use crate::{SyntaxKind, T};

pub(super) const PATTERN_FIRST: TokenSet = expressions::atom::LITERAL_FIRST
    .union(paths::PATH_FIRST)
    .union(TokenSet::new(&[
        // T![box],
        // T![ref],
        // T![mut],
        T!['('],
        // T!['['],
        // T![&],
        T!['_'],
        // T![-],
        T![..],
    ]));

pub(crate) fn pattern(p: &mut Parser) -> Option<CompletedMarker> {
    let completed = atom_pat(p, PAT_RECOVERY_SET);
    completed
}

pub(crate) fn ident_or_wildcard_pat(p: &mut Parser, recovery_set: TokenSet) -> Option<CompletedMarker> {
    let m = match p.current() {
        T![ident] => ident_pat(p),
        T!['_'] => wildcard_pat(p),
        _ => {
            p.error_and_bump_until_ts("expected ident or '_' pattern", recovery_set);
            return None;
        }
    };
    Some(m)
}

fn atom_pat(p: &mut Parser, recovery_set: TokenSet) -> Option<CompletedMarker> {
    let m = match p.current() {
        INT_NUMBER if p.nth_at(1, T![::]) => path_pat(p),
        IDENT => path_pat(p),
        // IDENT /*| INT_NUMBER if p.nth_at(1, T![::])*/ => match p.nth(1) {
        //     // Checks the token after an IDENT to see if a pattern is a path (Struct { .. }) or macro
        //     // (T![x]).
        //     T!['('] | T!['{'] | T![::] | T![<] => path_pat(p),
        //     // T![:] if p.nth_at(1, T![::]) => path_or_macro_pat(p),
        //     _ => ident_pat(p),
        // },

        // _ if is_literal_pat_start(p) => literal_pat(p),
        T![..] => rest_pat(p),
        // T![.] if p.at(T![..]) => rest_pat(p),
        T!['_'] => wildcard_pat(p),
        // T![&] => ref_pat(p),
        T!['('] => tuple_pat(p),
        // T!['['] => slice_pat(p),
        _ => {
            p.error_and_bump_until_ts("expected pattern", recovery_set);
            return None;
        }
    };

    Some(m)
}

fn path_pat(p: &mut Parser) -> CompletedMarker {
    match p.nth(1) {
        // Checks the token after an IDENT to see if a pattern is a path (Struct { .. }) or macro
        // (T![x]).
        T!['('] | T!['{'] | T![::] | T![<] => {
            assert!(paths::is_path_start(p));
            let m = p.start();
            paths::type_path(p);
            let kind = match p.current() {
                T!['('] => {
                    tuple_pat_fields(p);
                    TUPLE_STRUCT_PAT
                }
                T!['{'] => {
                    struct_pat_field_list(p);
                    STRUCT_PAT
                }
                _ => PATH_PAT,
            };
            m.complete(p, kind)
        }
        // T![:] if p.nth_at(1, T![::]) => path_or_macro_pat(p),
        _ => ident_pat(p),
    }
}

// // test path_part
// // fn foo() {
// //     let foo::Bar = ();
// //     let ::Bar = ();
// //     let Bar { .. } = ();
// //     let Bar(..) = ();
// // }
// fn path_pat(p: &mut Parser) -> CompletedMarker {
//     assert!(paths::is_path_start(p));
//     let m = p.start();
//     paths::expr_path(p);
//     let kind = match p.current() {
//         T!['('] => {
//             tuple_pat_fields(p);
//             TUPLE_STRUCT_PAT
//         }
//         T!['{'] => {
//             struct_pat_field_list(p);
//             STRUCT_PAT
//         }
//         // T![<] => {
//         //     opt_path_type_args(p);
//         //     PATH_PAT
//         // }
//         // test marco_pat
//         // fn main() {
//         //     let m!(x) = 0;
//         // }
//         // T![!] => {
//         //     items::macro_call_after_excl(p);
//         //     return m.complete(p, MACRO_CALL).precede(p).complete(p, MACRO_PAT);
//         // }
//         _ => PATH_PAT,
//     };
//     m.complete(p, kind)
// }

// test tuple_pat_fields
// fn foo() {
//     let S() = ();
//     let S(_) = ();
//     let S(_,) = ();
//     let S(_, .. , x) = ();
// }
fn tuple_pat_fields(p: &mut Parser) {
    assert!(p.at(T!['(']));
    p.bump(T!['(']);
    pat_list(p, T![')']);
    p.expect(T![')']);
}

// test record_pat_field
// fn foo() {
//     let S { 0: 1 } = ();
//     let S { x: 1 } = ();
//     let S { #[cfg(any())] x: 1 } = ();
// }
fn struct_pat_field(p: &mut Parser) {
    match p.current() {
        IDENT if p.nth(1) == T![:] => {
            name_ref(p);
            p.bump(T![:]);
            pattern(p);
        }
        T!['_'] => {
            wildcard_pat(p);
        }
        // IDENT | INT_NUMBER if p.nth(1) == T![:] => {
        //     name_ref_or_index(p);
        //     p.bump(T![:]);
        //     pattern(p);
        // }
        // T![..] => p.bump(T![..]),
        // T![.] => {
        //     if p.at(T![..]) {
        //         p.bump(T![..]);
        //     } else {
        //         ident_pat(p, false);
        //     }
        // }
        _ => {
            ident_pat(p);
        }
    }
}

// test record_pat_field_list
// fn foo() {
//     let S {} = ();
//     let S { f, ref mut g } = ();
//     let S { h: _, ..} = ();
//     let S { h: _, } = ();
//     let S { #[cfg(any())] .. } = ();
// }
fn struct_pat_field_list(p: &mut Parser) {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.bump(T!['{']);
    while !p.at(EOF) && !p.at(T!['}']) {
        let m = p.start();
        // attributes::outer_attrs(p);

        match p.current() {
            // A trailing `..` is *not* treated as a REST_PAT.
            T![..] => {
                // T![.] if p.at(T![..]) => {
                p.bump(T![..]);
                m.complete(p, REST_PAT);
            }
            T!['{'] => {
                error_block(p, "expected ident");
                m.abandon(p);
            }
            _ => {
                struct_pat_field(p);
                m.complete(p, STRUCT_PAT_FIELD);
            }
        }
        if !p.at(T!['}']) {
            p.expect(T![,]);
        }
    }
    p.expect(T!['}']);
    m.complete(p, STRUCT_PAT_FIELD_LIST);
}

// test placeholder_pat
// fn main() { let _ = (); }
fn wildcard_pat(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T!['_']));
    let m = p.start();
    p.bump(T!['_']);
    m.complete(p, WILDCARD_PAT)
}

// test dot_dot_pat
// fn main() {
//     let .. = ();
//     //
//     // Tuples
//     //
//     let (a, ..) = ();
//     let (a, ..,) = ();
//     let Tuple(a, ..) = ();
//     let Tuple(a, ..,) = ();
//     let (.., ..) = ();
//     let Tuple(.., ..) = ();
//     let (.., a, ..) = ();
//     let Tuple(.., a, ..) = ();
//     //
//     // Slices
//     //
//     let [..] = ();
//     let [head, ..] = ();
//     let [head, tail @ ..] = ();
//     let [head, .., cons] = ();
//     let [head, mid @ .., cons] = ();
//     let [head, .., .., cons] = ();
//     let [head, .., mid, tail @ ..] = ();
//     let [head, .., mid, .., cons] = ();
// }
fn rest_pat(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T![..]));
    let m = p.start();
    p.bump(T![..]);
    m.complete(p, REST_PAT)
}

// test tuple_pat
// fn main() {
//     let (a, b, ..) = ();
//     let (a,) = ();
//     let (..) = ();
//     let () = ();
// }
fn tuple_pat(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T!['(']));
    let m = p.start();
    p.bump(T!['(']);
    // let mut has_comma = false;
    // let mut has_pat = false;
    // let mut has_rest = false;
    while !p.at(EOF) && !p.at(T![')']) {
        // has_pat = true;
        if !p.at_ts(PATTERN_FIRST) {
            p.error("expected a pattern");
            break;
        }
        // has_rest |= p.at(T![..]);

        pattern(p);
        if !p.at(T![')']) {
            // has_comma = true;
            p.expect(T![,]);
        }
    }
    p.expect(T![')']);

    m.complete(p, TUPLE_PAT)
    // m.complete(p, if !has_comma && !has_rest && has_pat { PAREN_PAT } else { TUPLE_PAT })
}

fn pat_list(p: &mut Parser, ket: SyntaxKind) {
    while !p.at(EOF) && !p.at(ket) {
        if !p.at_ts(PATTERN_FIRST) {
            p.error("expected a pattern");
            break;
        }
        pattern(p);
        if !p.at(ket) {
            p.expect(T![,]);
        }
    }
}

// test bind_pat
// fn main() {
//     let a = ();
//     let mut b = ();
//     let ref c = ();
//     let ref mut d = ();
//     let e @ _ = ();
//     let ref mut f @ g @ _ = ();
// }
pub(crate) fn ident_pat(p: &mut Parser) -> CompletedMarker {
    let m = p.start();
    name(p);
    m.complete(p, IDENT_PAT)
}

pub(crate) const PAT_RECOVERY_SET: TokenSet = TokenSet::new(&[
    T![let],
    T![spec],
    T![if],
    T![while],
    T![loop],
    T![match],
    T![')'],
    T![,],
    T![=],
]);
