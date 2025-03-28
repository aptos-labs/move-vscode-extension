use crate::grammar::expressions::atom::{block_or_inline_expr, EXPR_FIRST};
use crate::grammar::items::{spec_inline_function, use_item};
use crate::grammar::params::lambda_param_list;
use crate::grammar::patterns::pattern;
use crate::grammar::specs::predicates::{pragma_stmt, spec_predicate, update_stmt};
use crate::grammar::specs::quants::{choose_expr, exists_expr, forall_expr, is_at_quant_kw};
use crate::grammar::specs::schemas::{apply_schema, global_variable, include_schema, schema_field};
use crate::grammar::utils::{delimited, list};
use crate::grammar::{
    error_block, name_ref, name_ref_or_index, opt_ret_type, paths, patterns, type_args, types,
    IDENT_OR_INT_NUMBER,
};
use crate::parser::{CompletedMarker, Marker, Parser};
use crate::token_set::TokenSet;
use crate::SyntaxKind::*;
use crate::{SyntaxKind, T};

pub(crate) mod atom;

pub(crate) fn expr(p: &mut Parser) -> bool {
    let r = Restrictions {
        forbid_structs: false,
        prefer_stmt: false,
    };
    expr_bp(p, None, r, 1).is_some()
}

// Parses expression with binding power of at least bp.
fn expr_bp(
    p: &mut Parser,
    m: Option<Marker>,
    mut r: Restrictions,
    bp: u8,
) -> Option<(CompletedMarker, BlockLike)> {
    let m = m.unwrap_or_else(|| {
        let m = p.start();
        // attributes::outer_attrs(p);
        m
    });
    let mut lhs = match lhs(p, r) {
        Some((lhs, blocklike)) => {
            let lhs = lhs.extend_to(p, m);
            if r.prefer_stmt && blocklike.is_block() {
                // test stmt_bin_expr_ambiguity
                // fn f() {
                //     let _ = {1} & 2;
                //     {1} &2;
                // }
                return Some((lhs, BlockLike::Block));
            }
            lhs
        }
        None => {
            m.abandon(p);
            return None;
        }
    };

    loop {
        let is_range = p.at(T![..]) /*|| p.at(T![..=])*/;
        let (op_bp, op) = current_op(p);
        if op_bp < bp {
            break;
        }
        // test as_precedence
        // fn f() { let _ = &1 as *const i32; }
        if p.at(T![as]) {
            lhs = cast_expr(p, lhs);
            continue;
        }
        if p.at_contextual_kw_ident("is") {
            lhs = is_expr(p, lhs);
            continue;
        }
        let m = lhs.precede(p);
        p.bump(op);

        // test binop_resets_statementness
        // fn f() { v = {1}&2; }
        r = Restrictions {
            prefer_stmt: false,
            ..r
        };

        if is_range {
            // test postfix_range
            // fn foo() {
            //     let x = 1..;
            //     match 1.. { _ => () };
            //     match a.b()..S { _ => () };
            // }
            let has_trailing_expression = p.at_ts(EXPR_FIRST) && !(r.forbid_structs && p.at(T!['{']));
            if !has_trailing_expression {
                // no RHS
                lhs = m.complete(p, RANGE_EXPR);
                break;
            }
        }

        expr_bp(
            p,
            None,
            Restrictions {
                prefer_stmt: false,
                ..r
            },
            op_bp + 1,
        );
        lhs = m.complete(p, if is_range { RANGE_EXPR } else { BIN_EXPR });
    }
    Some((lhs, BlockLike::NotBlock))
}

fn is_expr(p: &mut Parser, lhs: CompletedMarker) -> CompletedMarker {
    let m = lhs.precede(p);
    p.bump_remap(T![is]);
    types::type_no_bounds(p);
    while p.eat(T![|]) {
        types::type_no_bounds(p);
    }
    m.complete(p, IS_EXPR)
}

// test cast_expr
// fn foo() {
//     82 as i32;
//     81 as i8 + 1;
//     79 as i16 - 1;
//     0x36 as u8 <= 0x37;
// }
fn cast_expr(p: &mut Parser, lhs: CompletedMarker) -> CompletedMarker {
    assert!(p.at(T![as]));
    let m = lhs.precede(p);
    p.bump(T![as]);
    // Use type_no_bounds(), because cast expressions are not
    // allowed to have bounds.
    types::type_no_bounds(p);
    m.complete(p, CAST_EXPR)
}

// // test path_expr
// // fn foo() {
// //     let _ = a;
// //     let _ = a::b;
// //     let _ = ::a::<b>;
// //     let _ = format!();
// // }
// fn path_expr(p: &mut Parser) -> Option<Marker> {
//     assert!(paths::is_path_start(p));
//     let m = p.start();
//     paths::expr_path(p);
//     match p.current() {
//         T!['{'] /*if !r.forbid_structs*/ => {
//             struct_lit_field_list(p);
//             m.complete(p, STRUCT_LIT);
//             None
//         }
//         // T![!] if !p.at(T![!=]) => {
//         //     let block_like = items::macro_call_after_excl(p);
//         //     complete(p, MACRO_CALL)
//         // }
//         _ => { Some(m) },
//     }
// }

// test record_lit
// fn foo() {
//     S {};
//     S { x, y: 32, };
//     S { x, y: 32, ..Default::default() };
//     TupleStruct { 0: 1 };
// }
pub(crate) fn struct_lit_field_list(p: &mut Parser) {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.bump(T!['{']);
    while !p.at(EOF) && !p.at(T!['}']) {
        let m = p.start();
        // attributes::outer_attrs(p);
        match p.current() {
            IDENT /*| INT_NUMBER*/ => {
                // test_err record_literal_before_ellipsis_recovery
                // fn main() {
                //     S { field ..S::default() }
                // }
                if p.nth_at(1, T![:])
                /* || p.nth_at(1, T![..])*/
                {
                    name_ref(p);
                    p.expect(T![:]);
                }
                expr(p);
                m.complete(p, STRUCT_LIT_FIELD);
            }
            // T![.] if p.at(T![..]) => {
            //     m.abandon(p);
            //     p.bump(T![..]);
            //     expr(p);
            // }
            T!['{'] => {
                error_block(p, "expected a field");
                m.abandon(p);
            }
            _ => {
                p.err_and_bump("expected identifier");
                m.abandon(p);
            }
        }
        if !p.at(T!['}']) {
            p.expect(T![,]);
        }
    }
    p.expect(T!['}']);
    m.complete(p, STRUCT_LIT_FIELD_LIST);
}

const LHS_FIRST: TokenSet = atom::ATOM_EXPR_FIRST.union(TokenSet::new(&[T![&], T![*], T![!]]));

pub(crate) fn lhs(p: &mut Parser, r: Restrictions) -> Option<(CompletedMarker, BlockLike)> {
    let m;
    let kind = match p.current() {
        // test ref_expr
        // fn foo() {
        //     // reference operator
        //     let _ = &1;
        //     let _ = &mut &f();
        //     let _ = &raw;
        //     let _ = &raw.0;
        //     // raw reference operator
        //     let _ = &raw mut foo;
        //     let _ = &raw const foo;
        // }
        T![&] => {
            m = p.start();
            p.bump(T![&]);
            p.eat(T![mut]);
            BORROW_EXPR
        }
        T![|] => {
            m = p.start();
            if !lambda_param_list(p) {
                m.abandon(p);
                return None;
            }
            // p.bump(T![|]);
            // if p.at(T![,]) {
            //     m.abandon(p);
            //     return None;
            // }
            // delimited(
            //     p,
            //     T![,],
            //     || "expected parameter".into(),
            //     |p| p.at(T![|]),
            //     TokenSet::new(&[IDENT]),
            //     |p| {
            //         let m = p.start();
            //         patterns::ident_pat(p);
            //         if p.at(T![:]) {
            //             types::ascription(p);
            //         }
            //         m.complete(p, LAMBDA_PARAM);
            //         true
            //     },
            // );
            // if !p.eat(T![|]) {
            //     m.abandon_with_rollback(p);
            //     return None;
            // }
            LAMBDA_EXPR
        }
        IDENT if is_at_quant_kw(p) => {
            if let Some(cm) = forall_expr(p) {
                return Some((cm, BlockLike::NotBlock));
            }
            if let Some(cm) = exists_expr(p) {
                return Some((cm, BlockLike::NotBlock));
            }
            if let Some(cm) = choose_expr(p) {
                return Some((cm, BlockLike::NotBlock));
            }
            unreachable!()
        }
        // test unary_expr
        // fn foo() {
        //     **&1;
        //     !!true;
        //     --1;
        // }
        IDENT if p.at_contextual_kw("copy") => {
            m = p.start();
            p.bump_remap(T![copy]);
            RESOURCE_EXPR
        }
        T![move] => {
            m = p.start();
            p.bump(T![move]);
            RESOURCE_EXPR
        }
        T![*] => {
            m = p.start();
            p.bump(T![*]);
            DEREF_EXPR
        }
        T![!] => {
            m = p.start();
            p.bump(T![!]);
            BANG_EXPR
        }
        // T![*] | T![!] | T![move] => {
        //     m = p.start();
        //     p.bump_any();
        //     PREFIX_EXPR
        // }
        _ => {
            // test full_range_expr
            // fn foo() { xs[..]; }
            for op in [/*T![..=], */ T![..]] {
                if p.at(op) {
                    m = p.start();
                    p.bump(op);
                    if p.at_ts(EXPR_FIRST) && !(r.forbid_structs && p.at(T!['{'])) {
                        expr_bp(p, None, r, 2);
                    }
                    let cm = m.complete(p, RANGE_EXPR);
                    // return Some(cm);
                    return Some((cm, BlockLike::NotBlock));
                }
            }

            // test expression_after_block
            // fn foo() {
            //    let mut p = F{x: 5};
            //    {p}.x = 10;
            // }
            // let (lhs, blocklike) = atom::atom_expr(p, r)?;
            let (lhs, blocklike) = atom::atom_expr(p)?;
            let cm = postfix_expr(p, lhs, blocklike, !(r.prefer_stmt && blocklike.is_block()), false);
            // let cm = postfix_expr(p, lhs, !r.prefer_stmt);
            // let (cm, block_like) =
            //     postfix_expr(p, lhs, blocklike, !(r.prefer_stmt && blocklike.is_block()));
            return Some(cm);
        }
    };
    // parse the interior of the unary expression
    expr_bp(p, None, r, 255);
    let cm = m.complete(p, kind);
    Some((cm, BlockLike::NotBlock))
}

fn postfix_expr(
    p: &mut Parser<'_>,
    mut lhs: CompletedMarker,
    // Calls are disallowed if the type is a block and we prefer statements because the call cannot be disambiguated from a tuple
    // E.g. `while true {break}();` is parsed as
    // `while true {break}; ();`
    mut block_like: BlockLike,
    mut allow_calls: bool,
    is_schema: bool,
) -> (CompletedMarker, BlockLike) {
    loop {
        lhs = match p.current() {
            // test stmt_postfix_expr_ambiguity
            // fn foo() {
            //     match () {
            //         _ => {}
            //         () => {}
            //         [] => {}
            //     }
            // }
            // T!['('] if allow_calls => call_expr(p, lhs),
            // T![:] => {
            //     let m = lhs.precede(p);
            //     types::ascription(p);
            //     m.complete(p, ANNOTATED_EXPR)
            // }
            T!['['] if allow_calls => index_expr(p, lhs),
            T![.] => match postfix_dot_expr(p, lhs) {
                Ok(it) => it,
                Err(it) => {
                    lhs = it;
                    break;
                }
            },
            _ => break,
        };
        allow_calls = true;
        block_like = BlockLike::NotBlock;
    }
    (lhs, block_like)
}

const PATH_NAME_REF_KINDS: TokenSet = TokenSet::new(&[IDENT]);

fn postfix_dot_expr(
    p: &mut Parser<'_>,
    lhs: CompletedMarker,
) -> Result<CompletedMarker, CompletedMarker> {
    assert!(p.at(T![.]));
    // if !FLOAT_RECOVERY {
    // }
    let nth1 = 1;
    let nth2 = 2;

    if p.nth_at(1, IDENT) && (p.nth(nth2) == T!['('] || p.nth_at(nth2, T![::])) {
        return Ok(method_call_expr(p, lhs));
    }

    // test await_expr
    // fn foo() {
    //     x.await;
    //     x.0.await;
    //     x.0().await?.hello();
    //     x.0.0.await;
    //     x.0. await;
    // }
    // if p.nth(nth1) == T![await] {
    //     let m = lhs.precede(p);
    //     // if !FLOAT_RECOVERY {
    //     //     p.bump(T![.]);
    //     // }
    //     p.bump(T![await]);
    //     return Ok(m.complete(p, AWAIT_EXPR));
    // }

    // if p.at(T![..=]) || p.at(T![..]) {
    //     return Err(lhs);
    // }

    dot_expr(p, lhs)
}

// test method_call_expr
// fn foo() {
//     x.foo();
//     y.bar::<T>(1, 2,);
//     x.0.0.call();
//     x.0. call();
//     x.0()
// }
fn method_call_expr(p: &mut Parser<'_>, lhs: CompletedMarker) -> CompletedMarker {
    assert!(p.at(T![.]) && p.nth_at(1, IDENT) && (p.nth(2) == T!['('] || p.nth_at(2, T![::])));
    let m = lhs.precede(p);
    p.bump(T![.]);
    name_ref(p);
    type_args::opt_type_arg_list_for_expr(p, true);
    if p.at(T!['(']) {
        arg_list(p);
    } else {
        // emit an error when argument list is missing
        p.error("expected argument list");
    }
    m.complete(p, METHOD_CALL_EXPR)
}

// test field_expr
// fn foo() {
//     x.self;
//     x.Self;
//     x.foo;
//     x.0.bar;
//     x.0.1;
//     x.0. bar;
//     x.0();
// }
fn dot_expr(p: &mut Parser<'_>, lhs: CompletedMarker) -> Result<CompletedMarker, CompletedMarker> {
    assert!(p.at(T![.]));
    let m = lhs.precede(p);
    p.bump(T![.]);
    {
        let m = p.start();
        if p.at(IDENT) {
            let m = p.start();
            p.bump(IDENT);
            m.complete(p, NAME_REF);
        } else if p.at(INT_NUMBER) {
            let m = p.start();
            p.bump(INT_NUMBER);
            m.complete(p, INDEX_REF);
        } else {
            p.error("expected field name or number");
        }
        m.complete(p, FIELD_REF);
    }
    Ok(m.complete(p, DOT_EXPR))
}

// test index_expr
// fn foo() {
//     x[1][2];
// }
fn index_expr(p: &mut Parser<'_>, lhs: CompletedMarker) -> CompletedMarker {
    assert!(p.at(T!['[']));
    let m = lhs.precede(p);
    p.bump(T!['[']);
    expr(p);
    p.expect(T![']']);
    m.complete(p, INDEX_EXPR)
}

// // test call_expr
// // fn foo() {
// //     let _ = f();
// //     let _ = f()(1)(1, 2,);
// //     let _ = f(<Foo>::func());
// //     f(<Foo as Trait>::func());
// // }
// fn call_expr(p: &mut Parser<'_>) -> CompletedMarker {
//     assert!(p.at(T!['(']));
//     let m = p.start();
//     // path_expr(p);
//     arg_list(p);
//     m.complete(p, CALL_EXPR)
// }

// test_err arg_list_recovery
// fn main() {
//     foo(bar::);
//     foo(bar:);
//     foo(bar+);
//     foo(a, , b);
// }
fn arg_list(p: &mut Parser<'_>) {
    assert!(p.at(T!['(']));
    let m = p.start();
    list(
        p,
        T!['('],
        T![')'],
        T![,],
        || "expected expression".into(),
        EXPR_FIRST,
        // EXPR_FIRST.union(ATTRIBUTE_FIRST),
        |p| expr(p),
    );
    m.complete(p, ARG_LIST);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockLike {
    Block,
    NotBlock,
}

impl BlockLike {
    fn is_block(self) -> bool {
        self == BlockLike::Block
    }

    fn is_blocklike(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            /*SPEC_BLOCK_EXPR | */
            BLOCK_EXPR | IF_EXPR | WHILE_EXPR | FOR_EXPR | LOOP_EXPR | MATCH_EXPR
        )
    }
}

pub(super) enum StmtWithSemi {
    Yes,
    No,
    Optional,
}

pub(super) fn stmt(p: &mut Parser, with_semi: StmtWithSemi, prefer_expr: bool, is_spec: bool) {
    let m = p.start();
    // test attr_on_expr_stmt
    // fn foo() {
    //     #[A] foo();
    //     #[B] bar!{}
    //     #[C] #[D] {}
    //     #[D] return ();
    // }
    // attributes::outer_attrs(p);

    if p.at(T![let]) {
        let_stmt(p, m, with_semi);
        return;
    }
    if p.at(T![use]) {
        use_item::use_(p, m);
        return;
    }

    if is_spec {
        if p.at(T![native]) && p.nth_at(1, T![fun]) || p.at(T![fun]) {
            spec_inline_function(p);
            m.abandon(p);
            return;
        }
        // enable stmt level items unique to specs
        let spec_only_stmts = vec![
            schema_field,
            global_variable,
            pragma_stmt,
            update_stmt,
            include_schema,
            apply_schema,
            spec_predicate,
        ];
        if spec_only_stmts.iter().any(|spec_stmt| spec_stmt(p)) {
            m.abandon(p);
            return;
        }
    }

    if let Some((cm, blocklike)) = stmt_expr(p, Some(m)) {
        if !(p.at(T!['}']) || (prefer_expr && p.at(EOF))) {
            let m = cm.precede(p);
            match with_semi {
                StmtWithSemi::No => (),
                StmtWithSemi::Optional => {
                    p.eat(T![;]);
                }
                StmtWithSemi::Yes => {
                    p.expect(T![;]);
                    // if blocklike.is_block() {
                    //     p.eat(T![;]);
                    // } else {
                    // }
                }
            }

            m.complete(p, EXPR_STMT);
        }
    }

    // test let_stmt
    // fn f() { let x: i32 = 92; }
    fn let_stmt(p: &mut Parser, m: Marker, with_semi: StmtWithSemi) {
        p.bump(T![let]);
        if p.at_contextual_kw_ident("post") {
            p.bump_remap(T![post]);
        }
        pattern(p);
        if p.at(T![:]) {
            types::ascription(p);
        }
        opt_initializer_expr(p);

        match with_semi {
            StmtWithSemi::No => (),
            StmtWithSemi::Optional => {
                p.eat(T![;]);
            }
            StmtWithSemi::Yes => {
                p.expect(T![;]);
            }
        }
        m.complete(p, LET_STMT);
    }
}

pub(crate) fn opt_initializer_expr(p: &mut Parser) {
    if p.eat(T![=]) {
        if !expr(p) {
            p.error("expected expression");
        }
    }
}

pub(super) fn stmt_expr(p: &mut Parser, m: Option<Marker>) -> Option<(CompletedMarker, BlockLike)> {
    let r = Restrictions {
        forbid_structs: false,
        prefer_stmt: true,
    };
    expr_bp(p, m, r, 1)
}

pub(super) fn expr_block_contents(p: &mut Parser, is_spec: bool) {
    // attributes::inner_attrs(p);

    while !p.at(EOF) && !p.at(T!['}']) {
        // test nocontentexpr
        // fn foo(){
        //     ;;;some_expr();;;;{;;;};;;;Ok(())
        // }

        // test nocontentexpr_after_item
        // fn simple_function() {
        //     enum LocalEnum {
        //         One,
        //         Two,
        //     };
        //     fn f() {};
        //     struct S {};
        // }

        if p.at(T![;]) {
            p.bump(T![;]);
            continue;
        }

        stmt(p, StmtWithSemi::Yes, false, is_spec);
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) struct Restrictions {
    forbid_structs: bool,
    prefer_stmt: bool,
}

/// Binding powers of operators for a Pratt parser.
///
/// See <https://matklad.github.io/2020/04/13/simple-but-powerful-pratt-parsing.html>
#[rustfmt::skip]
fn current_op(p: &Parser) -> (u8, SyntaxKind) {
    const NOT_AN_OP: (u8, SyntaxKind) = (0, T![@]);
    match p.current() {
        // T![||]  => (3,  T![||]),
        T![|] if p.at(T![||])  => (3,  T![||]),
        T![|=]  => (1,  T![|=]),
        // T![|] if p.at(T![|=])  => (1,  T![|=]),
        T![|]                  => (6,  T![|]),
        T![>] if p.at(T![>>=]) => (1,  T![>>=]),
        T![>] if p.at(T![>>])  => (9,  T![>>]),
        T![>] if p.at(T![>=])  => (5,  T![>=]),
        T![>]                  => (5,  T![>]),
        T![=] if p.at(T![=>])  => NOT_AN_OP,
        T![==>]  => (1, T![==>]),
        T![<] if p.at(T![<==>])  => (1, T![<==>]),
        T![==]  => (5,  T![==]),
        // T![=] if p.at(T![==])  => (5,  T![==]),
        T![=]                  => (1,  T![=]),
        T![<] if p.at(T![<=])  => (5,  T![<=]),
        T![<] if p.at(T![<<=]) => (1,  T![<<=]),
        T![<] if p.at(T![<<])  => (9,  T![<<]),
        T![<]                  => (5,  T![<]),
        T![+=]  => (1,  T![+=]),
        // T![+] if p.at(T![+=])  => (1,  T![+=]),
        T![+]                  => (10, T![+]),
        T![^=]  => (1,  T![^=]),
        // T![^] if p.at(T![^=])  => (1,  T![^=]),
        T![^]                  => (7,  T![^]),
        T![%=]  => (1,  T![%=]),
        // T![%] if p.at(T![%=])  => (1,  T![%=]),
        T![%]                  => (11, T![%]),
        T![&=]  => (1,  T![&=]),
        // T![&] if p.at(T![&=])  => (1,  T![&=]),
        // T![&&]  => (4,  T![&&]),
        T![&] if p.at(T![&&])  => (4,  T![&&]),
        T![&]                  => (8,  T![&]),
        T![/=]  => (1,  T![/=]),
        // T![/] if p.at(T![/=])  => (1,  T![/=]),
        T![/]                  => (11, T![/]),
        T![*=]  => (1,  T![*=]),
        // T![*] if p.at(T![*=])  => (1,  T![*=]),
        T![*]                  => (11, T![*]),
        // T![.] if p.at(T![..=]) => (2,  T![..=]),
        T![..]  => (2,  T![..]),
        // T![.] if p.at(T![..])  => (2,  T![..]),
        T![!=]  => (5,  T![!=]),
        // T![!] if p.at(T![!=])  => (5,  T![!=]),
        T![-=]  => (1,  T![-=]),
        // T![-] if p.at(T![-=])  => (1,  T![-=]),
        T![-]                  => (10, T![-]),
        T![as]                 => (12, T![as]),
        T![ident] if p.at_contextual_kw("is") => (12, T![is]),
        _                      => NOT_AN_OP
    }
}
