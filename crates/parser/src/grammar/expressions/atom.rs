use super::*;
use crate::grammar::paths::Mode;
use crate::grammar::specs::{opt_spec_block_expr, spec_block_expr};
use crate::grammar::{address_ref, paths};
use crate::token_set::TokenSet;

// test expr_literals
// fn foo() {
//     let _ = true;
//     let _ = false;
//     let _ = 1;
//     let _ = 2.0;
//     let _ = b'a';
//     let _ = 'b';
//     let _ = "c";
//     let _ = r"d";
//     let _ = b"e";
//     let _ = br"f";
// }
pub(crate) const LITERAL_FIRST: TokenSet =
    TokenSet::new(&[T![true], T![false], INT_NUMBER, T![@], BYTE_STRING]);

pub(crate) fn literal(p: &mut Parser) -> Option<CompletedMarker> {
    let m = p.start();
    match p.current() {
        // 0x1::m
        INT_NUMBER if p.nth_at(1, T![::]) => {
            m.abandon(p);
            return None;
        }
        T![@] => {
            p.bump(T![@]);
            address_ref(p);
        }
        INT_NUMBER | BYTE_STRING | HEX_STRING | T![true] | T![false] => {
            p.bump_any();
        }
        _ => {
            m.abandon(p);
            return None;
        }
    }
    Some(m.complete(p, LITERAL))
}

// E.g. for after the break in `if break {}`, this should not match
pub(super) const ATOM_EXPR_FIRST: TokenSet =
    LITERAL_FIRST.union(paths::PATH_FIRST).union(TokenSet::new(&[
        T!['('],
        T!['{'],
        T!['['],
        T![|],
        T![move],
        T![if],
        T![while],
        // T![match],
        T![return],
        T![break],
        T![continue],
        T![const],
        T![loop],
        T![for],
        T![copy],
        T![move],
        QUOTE_IDENT,
    ]));

const EXPR_RECOVERY_SET: TokenSet = TokenSet::new(&[T![let]]);

pub(crate) fn atom_expr(p: &mut Parser) -> Option<(CompletedMarker, BlockLike)> {
    if p.at(T!['(']) && p.nth_at(1, T![')']) {
        let m = p.start();
        p.bump(T!['(']);
        p.bump(T![')']);
        return Some((m.complete(p, UNIT_EXPR), BlockLike::NotBlock));
    }
    if let Some(m) = literal(p) {
        return Some((m, BlockLike::NotBlock));
    }
    if p.at(IDENT) && p.at_contextual_kw("vector") && (p.nth_at(1, T![<]) || p.nth_at(1, T!['['])) {
        // vector[1, 2]
        let m = p.start();
        p.bump(IDENT);
        paths::opt_path_args(p, Mode::Type);
        list(
            p,
            T!['['],
            T![']'],
            T![,],
            || "expected comma".into(),
            EXPR_FIRST,
            |p| expr(p),
        );
        return Some((m.complete(p, VECTOR_LIT_EXPR), BlockLike::NotBlock));
    }

    if paths::is_path_start(p) && !(p.at_contextual_kw("for") && p.nth_at(1, T!['('])) {
        // special case for match
        if p.at_contextual_kw("match") && p.nth_at(1, T!['(']) {
            let m = p.start();
            p.bump_remap(T![match]);
            list(
                p,
                T!['('],
                T![')'],
                T![,],
                || "expected expression".into(),
                EXPR_FIRST,
                |p| expr(p),
            );
            if p.at(T!['{']) {
                match_arm_list(p);
                return Some((m.complete(p, MATCH_EXPR), BlockLike::Block));
            } else {
                m.abandon_with_rollback(p);
            }
        }
        if p.at(IDENT) && p.at_contextual_kw("assert") && p.nth_at(1, T![!]) {
            let m = p.start();
            p.bump(IDENT);
            p.bump(T![!]);
            arg_list(p);
            return Some((m.complete(p, ASSERT_MACRO_EXPR), BlockLike::NotBlock));
        }
        let m = p.start();
        paths::expr_path(p);
        let cm = match p.current() {
            T!['{'] /*if !r.forbid_structs*/ => {
                struct_lit_field_list(p);
                m.complete(p, STRUCT_LIT)
            }
            T!['('] => {
                arg_list(p);
                m.complete(p, CALL_EXPR)
            }
            // T![!] if !p.at(T![!=]) => {
            //     let block_like = items::macro_call_after_excl(p);
            //     complete(p, MACRO_CALL)
            // }
            _ => { m.complete(p, PATH_EXPR) },
        };
        return Some((cm, BlockLike::NotBlock));
    }
    let done = match p.current() {
        T!['('] => paren_or_tuple_or_annotated_expr(p),
        T![spec] => spec_block_expr(p),
        //     T!['['] => array_expr(p),
        //     T![|] => closure_expr(p),

        //     T![async] if la == T![|] || (la == T![move] && p.nth(2) == T![|]) => closure_expr(p),
        T![if] => if_expr(p),
        T![loop] => loop_expr(p, None),
        //     T![box] => box_expr(p, None),
        IDENT if p.at_contextual_kw("for") => for_expr(p, None),
        T![while] => while_expr(p, None),
        //     T![try] => try_block_expr(p, None),
        QUOTE_IDENT if p.nth(1) == T![:] => {
            let m = p.start();
            label(p);
            match p.current() {
                T![loop] => loop_expr(p, Some(m)),
                IDENT if p.at_contextual_kw("for") => for_expr(p, Some(m)),
                T![while] => while_expr(p, Some(m)),
                // test labeled_block
                // fn f() { 'label: {}; }
                // T!['{'] => {
                //     stmt_list(p, false);
                //     m.complete(p, BLOCK_EXPR)
                // }
                _ => {
                    // test_err misplaced_label_err
                    // fn main() {
                    //     'loop: impl
                    // }
                    p.error("expected a loop");
                    m.complete(p, ERROR);
                    return None;
                }
            }
        }
        //     }
        //     T![async] if la == T!['{'] || (la == T![move] && p.nth(2) == T!['{']) => {
        //         let m = p.start();
        //         p.bump(T![async]);
        //         p.eat(T![move]);
        //         stmt_list(p);
        //         m.complete(p, BLOCK_EXPR)
        //     }
        // IDENT if p.at_contextual_kw("match") && p.nth_at(1, T!['(']) => match_expr(p),
        //     // test unsafe_block
        //     // fn f() { unsafe { } }
        //     T![unsafe] if la == T!['{'] => {
        //         let m = p.start();
        //         p.bump(T![unsafe]);
        //         stmt_list(p);
        //         m.complete(p, BLOCK_EXPR)
        //     }
        //     // test const_block
        //     // fn f() { const { } }
        //     T![const] if la == T!['{'] => {
        //         let m = p.start();
        //         p.bump(T![const]);
        //         stmt_list(p);
        //         m.complete(p, BLOCK_EXPR)
        //     }
        T!['{'] => {
            // test for_range_from
            // fn foo() {
            //    for x in 0 .. {
            //        break;
            //    }
            // }
            let m = p.start();
            stmt_list(p, false);
            m.complete(p, BLOCK_EXPR)
        }
        T![return] => return_expr(p),
        T![abort] => abort_expr(p),
        T![continue] => continue_expr(p),
        T![break] => break_expr(p),
        _ => {
            p.err_and_bump("expected expression");
            // p.err_and_bump("expected expression", EXPR_RECOVERY_SET);
            return None;
        }
    };
    let blocklike = if (BlockLike::is_blocklike(done.kind())) {
        BlockLike::Block
    } else {
        BlockLike::NotBlock
    };
    Some((done, blocklike))
}

// test tuple_expr
// fn foo() {
//     ();
//     (1);
//     (1,);
// }
fn paren_or_tuple_or_annotated_expr(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T!['(']));
    let m = p.start();
    p.expect(T!['(']);

    let mut first = true;
    let mut saw_comma = false;
    let mut saw_expr = false;
    while !p.at(EOF) && !p.at(T![')']) {
        saw_expr = true;

        if !expr(p) {
            break;
        }

        // try for `(a: u8)` annotated expr
        if first {
            if p.at(T![:]) {
                types::ascription(p);
                p.expect(T![')']);
                return m.complete(p, ANNOTATED_EXPR);
            }
            first = false;
        }

        if !p.at(T![')']) {
            saw_comma = true;
            p.expect(T![,]);
        }
    }
    p.expect(T![')']);
    m.complete(
        p,
        if saw_expr && !saw_comma {
            PAREN_EXPR
        } else {
            TUPLE_EXPR
        },
    )
}

// test if_expr
// fn foo() {
//     if true {};
//     if true {} else {};
//     if true {} else if false {} else {};
//     if S {};
//     if { true } { } else { };
// }
fn if_expr(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T![if]));
    let m = p.start();
    p.bump(T![if]);
    condition(p);
    block_or_inline_expr(p, false);
    if p.at(T![else]) {
        p.bump(T![else]);
        if p.at(T![if]) {
            if_expr(p);
        } else {
            block_or_inline_expr(p, false);
        }
    }
    m.complete(p, IF_EXPR)
}

// test label
// fn foo() {
//     'a: loop {}
//     'b: while true {}
//     'c: for x in () {}
// }
fn label(p: &mut Parser<'_>) {
    assert!(p.at(QUOTE_IDENT) && p.nth(1) == T![:]);
    let m = p.start();
    p.bump(QUOTE_IDENT);
    p.bump(T![:]);
    m.complete(p, LABEL);
}

// test loop_expr
// fn foo() {
//     loop {};
// }
fn loop_expr(p: &mut Parser<'_>, m: Option<Marker>) -> CompletedMarker {
    assert!(p.at(T![loop]));
    let m = m.unwrap_or_else(|| p.start());
    p.bump(T![loop]);
    block_or_inline_expr(p, false);
    m.complete(p, LOOP_EXPR)
}

// test for_expr
// fn foo() {
//     for (x in 0..10) {};
// }
fn for_expr(p: &mut Parser, m: Option<Marker>) -> CompletedMarker {
    assert!(p.at(IDENT) && p.at_contextual_kw("for"));
    let m = m.unwrap_or_else(|| p.start());
    p.bump_remap(T![for]);
    for_condition(p);
    block_or_inline_expr(p, false);
    m.complete(p, FOR_EXPR)
}

fn for_condition(p: &mut Parser) {
    // todo: recovery
    let m = p.start();
    p.expect(T!['(']);
    patterns::ident_pat(p);
    if p.at(IDENT) && p.at_contextual_kw("in") {
        p.bump_remap(T![in]);
        expr(p);
    } else {
        p.err_recover("expected 'in'", TokenSet::new(&[T![')']]));
    }
    opt_spec_block_expr(p);
    p.expect(T![')']);
    m.complete(p, FOR_CONDITION);
}

// test while_expr
// fn foo() {
//     while true {};
//     while let Some(x) = it.next() {};
//     while { true } {};
// }
fn while_expr(p: &mut Parser<'_>, m: Option<Marker>) -> CompletedMarker {
    assert!(p.at(T![while]));
    let m = m.unwrap_or_else(|| p.start());
    p.bump(T![while]);
    condition(p);
    block_or_inline_expr(p, false);
    opt_spec_block_expr(p);
    m.complete(p, WHILE_EXPR)
}

// test cond
// fn foo() { if let Some(_) = None {} }
// fn bar() {
//     if let Some(_) | Some(_) = None {}
//     if let | Some(_) = None {}
//     while let Some(_) | Some(_) = None {}
//     while let | Some(_) = None {}
// }
fn condition(p: &mut Parser) {
    let m = p.start();
    // if p.eat(T![let]) {
    //     patterns::pattern_top(p);
    //     p.expect(T![=]);
    // }
    p.expect(T!['(']);
    let r = Restrictions {
        forbid_structs: true,
        prefer_stmt: false,
    };
    expr_bp(p, None, r, 1);
    p.expect(T![')']);

    m.complete(p, CONDITION);
}

// // test match_expr
// // fn foo() {
// //     match () { };
// //     match S {};
// //     match { } { _ => () };
// //     match { S {} } {};
// // }
// fn match_expr(p: &mut Parser) -> CompletedMarker {
//     let m = p.start();
//     p.bump_remap(T![match]);
//     p.bump(T!['(']);
//     expr(p);
//     p.expect(T![')']);
//     if p.at(T!['{']) {
//         match_arm_list(p);
//     } else {
//         p.error("expected `{`");
//     }
//     m.complete(p, MATCH_EXPR)
// }

pub(crate) fn match_arm_list(p: &mut Parser) {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.eat(T!['{']);

    // test match_arms_inner_attribute
    // fn foo() {
    //     match () {
    //         #![doc("Inner attribute")]
    //         #![doc("Can be")]
    //         #![doc("Stacked")]
    //         _ => (),
    //     }
    // }
    // attributes::inner_attrs(p);

    while !p.at(EOF) && !p.at(T!['}']) {
        if p.at(T!['{']) {
            error_block(p, "expected match arm");
            continue;
        }
        match_arm(p);
    }
    p.expect(T!['}']);
    m.complete(p, MATCH_ARM_LIST);
}

// test match_arm
// fn foo() {
//     match () {
//         _ => (),
//         _ if Test > Test{field: 0} => (),
//         X | Y if Z => (),
//         | X | Y if Z => (),
//         | X => (),
//     };
// }
fn match_arm(p: &mut Parser) {
    let m = p.start();
    pattern(p);
    if p.at(T![if]) {
        match_guard(p);
    }
    p.expect(T![=>]);
    let blocklike = match expr_stmt(p, None) {
        Some((_, blocklike)) => blocklike,
        None => BlockLike::NotBlock,
    };

    // test match_arms_commas
    // fn foo() {
    //     match () {
    //         _ => (),
    //         _ => {}
    //         _ => ()
    //     }
    // }
    if !p.eat(T![,]) && !blocklike.is_block() && !p.at(T!['}']) {
        p.error("expected `,`");
    }
    m.complete(p, MATCH_ARM);
}

// test match_guard
// fn foo() {
//     match () {
//         _ if foo => (),
//         _ if let foo = bar => (),
//     }
// }
fn match_guard(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T![if]));
    let m = p.start();
    p.bump(T![if]);
    // if p.eat(T![let]) {
    //     patterns::pattern_top(p);
    //     p.expect(T![=]);
    // }
    expr(p);
    m.complete(p, MATCH_GUARD)
}

pub(crate) fn block_or_inline_expr(p: &mut Parser, is_spec: bool) {
    if p.at(T!['{']) {
        block_expr(p, is_spec);
    } else {
        inline_expr(p);
    }
}

// test block
// fn a() {}
// fn b() { let _ = 1; }
// fn c() { 1; 2; }
// fn d() { 1; 2 }
pub(crate) fn block_expr(p: &mut Parser, is_spec: bool) {
    if !p.at(T!['{']) {
        p.error("expected a block");
        return;
    }
    let m = p.start();
    stmt_list(p, is_spec);
    // let m = p.start();
    // p.bump(T!['{']);
    //
    // let r = Restrictions { forbid_structs: false, prefer_stmt: false };
    // expr_bp(p, None, r, 1);
    // p.expect(T!['}']);
    m.complete(p, BLOCK_EXPR);
}

pub(crate) fn inline_expr(p: &mut Parser) -> bool {
    assert!(!p.at(T!['{']));
    // if p.at(T!['{']) {
    //     p.error("expected a block");
    //     return;
    // }
    let m = p.start();
    let found = expr(p);
    // let m = p.start();
    // p.bump(T!['{']);
    //
    // let r = Restrictions { forbid_structs: false, prefer_stmt: false };
    // expr_bp(p, None, r, 1);
    // p.expect(T!['}']);
    m.complete(p, INLINE_EXPR);
    found
}

fn stmt_list(p: &mut Parser<'_>, is_spec: bool) -> CompletedMarker {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.bump(T!['{']);
    expr_block_contents(p, is_spec);
    p.expect(T!['}']);
    m.complete(p, STMT_LIST)
}

// test return_expr
// fn foo() {
//     return;
//     return 92;
// }
fn return_expr(p: &mut Parser<'_>) -> CompletedMarker {
    assert!(p.at(T![return]));
    let m = p.start();
    p.bump(T![return]);
    if p.at_ts(EXPR_FIRST) {
        expr(p);
    }
    m.complete(p, RETURN_EXPR)
}

fn abort_expr(p: &mut Parser<'_>) -> CompletedMarker {
    assert!(p.at(T![abort]));
    let m = p.start();
    p.bump(T![abort]);
    if p.at_ts(EXPR_FIRST) {
        expr(p);
    }
    m.complete(p, ABORT_EXPR)
}

// test continue_expr
// fn foo() {
//     loop {
//         continue;
//         continue 'l;
//     }
// }
fn continue_expr(p: &mut Parser<'_>) -> CompletedMarker {
    assert!(p.at(T![continue]));
    let m = p.start();
    p.bump(T![continue]);
    p.eat(QUOTE_IDENT);
    m.complete(p, CONTINUE_EXPR)
}

// test break_expr
// fn foo() {
//     loop {
//         break;
//         break 'l;
//         break 92;
//         break 'l 92;
//     }
// }
fn break_expr(p: &mut Parser<'_>) -> CompletedMarker {
    assert!(p.at(T![break]));
    let m = p.start();
    p.bump(T![break]);
    p.eat(QUOTE_IDENT);
    // test break_ambiguity
    // fn foo(){
    //     if break {}
    //     while break {}
    //     for i in break {}
    //     match break {}
    // }
    if p.at_ts(EXPR_FIRST)
    /*&& !(false && p.at(T!['{']))*/
    {
        expr(p);
    }
    m.complete(p, BREAK_EXPR)
}

pub(crate) const EXPR_FIRST: TokenSet = LHS_FIRST;

pub(crate) const IDENT_FIRST: TokenSet = TokenSet::new(&[IDENT]);
