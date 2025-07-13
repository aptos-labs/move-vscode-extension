// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::SyntaxKind::*;
use crate::parse::grammar::paths::PathMode;
use crate::parse::grammar::utils::delimited_with_recovery;
use crate::parse::grammar::{expressions, name, name_ref, paths};
use crate::parse::parser::{CompletedMarker, Parser};
use crate::parse::recovery_set::RecoverySet;
use crate::parse::token_set::TokenSet;
use crate::{SyntaxKind, T};
use std::ops::ControlFlow::{Break, Continue};

pub(crate) fn pat(p: &mut Parser) -> bool {
    pat_or_recover(p, TokenSet::EMPTY)
}

pub(crate) fn pat_or_recover(p: &mut Parser, extra_set: impl Into<RecoverySet>) -> bool {
    match p.current() {
        // 0x1 '::'
        INT_NUMBER if p.nth_at(1, T![::]) => path_pat(p),
        IDENT => path_pat(p),

        T![..] => rest_pat(p),
        T!['_'] => wildcard_pat(p),
        T!['('] => tuple_or_unit_or_paren_pat(p),

        _ => {
            p.error_and_recover("expected pattern", extra_set.into());
            return false;
        }
    };
    true
}

pub(crate) fn ident_pat_or_recover(p: &mut Parser) -> bool {
    match p.current() {
        T![ident] => ident_pat(p),
        T!['_'] => wildcard_pat(p),
        _ => {
            p.error_and_recover("expected ident", TokenSet::EMPTY);
            // p.error_and_recover_until_ts("expected ident or '_'", recovery_set);
            return false;
        }
    };
    true
}

fn path_pat(p: &mut Parser) -> CompletedMarker {
    match p.nth(1) {
        // Checks the token after an IDENT to see if a pattern is a path (Struct { .. }).
        T!['('] | T!['{'] | T![::] | T![<] => {
            assert!(paths::is_path_start(p));
            let m = p.start();
            paths::path(p, Some(PathMode::Type));
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
        _ => ident_pat(p),
    }
}

fn tuple_pat_fields(p: &mut Parser) {
    assert!(p.at(T!['(']));
    p.bump(T!['(']);

    delimited_with_recovery(p, pat, T![,], "expected pattern", Some(T![')']));

    // while !p.at(EOF) && !p.at(T![')']) {
    //     if !p.at_ts(PAT_FIRST) {
    //         p.error("expected a pattern");
    //         break;
    //     }
    //     pat_or_recover(p, TokenSet::EMPTY);
    //     if !p.at(T![')']) {
    //         p.expect(T![,]);
    //     }
    // }

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
        T![..] => {
            let m = p.start();
            p.bump(T![..]);
            m.complete(p, REST_PAT);
        }
        _ => {
            p.error_and_recover("expected identifier", TokenSet::EMPTY);
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

    p.with_recovery_token(T!['}'], |p| {
        delimited_with_recovery(
            p,
            |p| {
                let field_m = p.start();
                match p.current() {
                    // T![..] => {
                    //     let m = p.start();
                    //     p.bump(T![..]);
                    //     m.complete(p, REST_PAT);
                    //     return true;
                    // }
                    T!['}'] => {
                        // empty struct pat
                        field_m.abandon(p);
                        return true;
                    }
                    _ => {
                        let is_field = struct_pat_field(p);
                        field_m.complete(p, STRUCT_PAT_FIELD);
                        is_field
                    }
                }
            },
            T![,],
            "expected ident",
            Some(T!['}']),
        );
    });

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

fn tuple_or_unit_or_paren_pat(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T!['(']));
    let m = p.start();
    p.bump(T!['(']);
    let mut has_comma = false;
    let mut has_pat = false;

    let outer_recovery_set = p.outer_recovery_set();
    p.iterate_to_EOF(T![')'], |p| {
        let found_pat = pat_or_recover(p, T![,] | T![')']);
        if found_pat {
            has_pat = true;
        }

        if outer_recovery_set.contains_current(p) {
            return Break(());
        }

        if !p.at(T![')']) {
            if p.expect(T![,]) {
                has_comma = true;
            }
        }

        Continue(())
    });
    p.expect(T![')']);

    m.complete(
        p,
        if !has_pat && !has_comma {
            UNIT_PAT
        } else if has_pat && !has_comma {
            PAREN_PAT
        } else {
            TUPLE_PAT
        },
    )
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
            T![spec],
            T![use],
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
