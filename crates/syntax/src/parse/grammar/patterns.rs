use crate::parse::grammar::utils::delimited_with_recovery;
use crate::parse::grammar::{error_block, expressions, name, name_ref, paths};
use crate::parse::parser::{CompletedMarker, Parser};
use crate::parse::token_set::TokenSet;
use crate::SyntaxKind::*;
use crate::{SyntaxKind, T};
use std::cell::LazyCell;

pub(crate) fn pat(p: &mut Parser) -> Option<CompletedMarker> {
    let m = match p.current() {
        // 0x1 '::'
        INT_NUMBER if p.nth_at(1, T![::]) => path_pat(p),
        IDENT => path_pat(p),
        // _ if is_literal_pat_start(p) => literal_pat(p),
        T![..] => rest_pat(p),
        T!['_'] => wildcard_pat(p),
        T!['('] => tuple_pat(p),
        _ => {
            p.error_and_recover("expected pattern", TokenSet::EMPTY.into());
            // p.error_and_recover_until_ts("expected pattern", PAT_RECOVERY_SET);
            return None;
        }
    };
    Some(m)
}

pub(crate) fn ident_or_wildcard_pat(p: &mut Parser) -> bool {
    match p.current() {
        T![ident] => ident_pat(p),
        T!['_'] => wildcard_pat(p),
        _ => {
            return false;
        }
    };
    true
}

pub(crate) fn ident_or_wildcard_pat_or_recover(
    p: &mut Parser,
    recovery_set: TokenSet,
) -> Option<CompletedMarker> {
    let m = match p.current() {
        T![ident] => ident_pat(p),
        T!['_'] => wildcard_pat(p),
        _ => {
            p.error_and_recover_until_ts("expected ident or '_'", recovery_set);
            return None;
        }
    };
    Some(m)
}

fn path_pat(p: &mut Parser) -> CompletedMarker {
    match p.nth(1) {
        // Checks the token after an IDENT to see if a pattern is a path (Struct { .. }).
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

fn tuple_pat_fields(p: &mut Parser) {
    assert!(p.at(T!['(']));
    p.bump(T!['(']);
    pat_list(p, T![')']);
    p.expect(T![')']);
}

fn struct_pat_field(p: &mut Parser) -> bool {
    match p.current() {
        IDENT if p.nth(1) == T![:] => {
            name_ref(p);
            p.bump(T![:]);
            pat(p);
        }
        IDENT => {
            ident_pat(p);
        }
        T!['_'] => {
            wildcard_pat(p);
        }
        _ => {
            p.error_and_recover("expected identifier", TokenSet::EMPTY.into());
            // p.error_and_recover_until_ts("expected identifier", PAT_RECOVERY_SET);
            return false;
        }
    }
    true
}

fn struct_pat_field_list(p: &mut Parser) {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.bump(T!['{']);

    p.with_recover_token(T!['}'], |p| {
        delimited_with_recovery(
            p,
            |p| {
                let m = p.start();
                match p.current() {
                    // A trailing `..` is *not* treated as a REST_PAT.
                    T![..] => {
                        p.bump(T![..]);
                        m.complete(p, REST_PAT);
                        return true;
                    }
                    // T!['{'] => {
                    //     error_block(p, "expected ident");
                    //     m.abandon(p);
                    //     return false;
                    // }
                    T!['}'] => {
                        // empty struct pat
                        m.abandon(p);
                        return true;
                    }
                    _ => {
                        let res = struct_pat_field(p);
                        m.complete(p, STRUCT_PAT_FIELD);
                        res
                    }
                }
            },
            T![,],
            "expected ident",
            Some(T!['}']),
        );
    });

    // while !p.at(EOF) && !p.at(T!['}']) {
    //     let m = p.start();
    //     // attributes::outer_attrs(p);
    //     match p.current() {
    //         // A trailing `..` is *not* treated as a REST_PAT.
    //         T![..] => {
    //             // T![.] if p.at(T![..]) => {
    //             p.bump(T![..]);
    //             m.complete(p, REST_PAT);
    //             return;
    //         }
    //         T!['{'] => {
    //             error_block(p, "expected ident");
    //             m.abandon(p);
    //         }
    //         _ => {
    //             p.with_recover_token_set(T!['}'] | T![,], struct_pat_field);
    //             // struct_pat_field(p);
    //             m.complete(p, STRUCT_PAT_FIELD);
    //         }
    //     }
    //     if !p.at(T!['}']) {
    //         p.expect(T![,]);
    //     }
    // }
    p.expect(T!['}']);
    m.complete(p, STRUCT_PAT_FIELD_LIST);
}

fn wildcard_pat(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T!['_']));
    let m = p.start();
    p.bump(T!['_']);
    m.complete(p, WILDCARD_PAT)
}

fn rest_pat(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T![..]));
    let m = p.start();
    p.bump(T![..]);
    m.complete(p, REST_PAT)
}

fn tuple_pat(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T!['(']));
    let m = p.start();
    p.bump(T!['(']);
    // let mut has_comma = false;
    // let mut has_pat = false;
    // let mut has_rest = false;
    while !p.at(EOF) && !p.at(T![')']) {
        // has_pat = true;
        if !p.at_ts(PAT_FIRST) {
            p.error("expected a pattern");
            break;
        }
        // has_rest |= p.at(T![..]);

        p.with_recover_token_set(T![')'] | T![,], pat);
        // pat(p);
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
        if !p.at_ts(PAT_FIRST) {
            p.error("expected a pattern");
            break;
        }
        pat(p);
        if !p.at(ket) {
            p.expect(T![,]);
        }
    }
}

pub(crate) fn ident_pat(p: &mut Parser) -> CompletedMarker {
    let m = p.start();
    name(p);
    m.complete(p, IDENT_PAT)
}

pub(crate) const PAT_FIRST: TokenSet = expressions::atom::LITERAL_FIRST
    .union(paths::PATH_FIRST)
    .union(TokenSet::new(&[T!['('], T!['_'], T![..]]));

pub(crate) const EXPR_STMT_KEYWORDS_LIST: &[SyntaxKind] = &[T![if], T![while], T![loop], T![match]];

pub(crate) const STMT_KEYWORDS_LIST: &[SyntaxKind] =
    &[T![if], T![while], T![loop], T![match], T![let], T![spec]];

#[rustfmt::skip]
pub(crate) const EXPR_STMT_FIRST: TokenSet = TokenSet::new(&[
    T![if],
    T![while],
    T![loop],
    T![match]
]);

#[rustfmt::skip]
pub(crate) const STMT_FIRST: TokenSet =
    EXPR_STMT_FIRST.union(
        TokenSet::new(&[
            T![let],
            T![spec]
        ]));

#[rustfmt::skip]
pub(crate) const PAT_RECOVERY_SET: TokenSet =
    STMT_FIRST.union(
          TokenSet::new(&[
              T![')'],
              T!['}'],
              T![,],
              T![=]
          ]));
