use crate::parse::grammar::expressions::atom::{call_expr, EXPR_FIRST};
use crate::parse::grammar::items::{fun, use_item};
use crate::parse::grammar::lambdas::lambda_param_list;
use crate::parse::grammar::patterns::pattern;
use crate::parse::grammar::specs::predicates::{pragma_stmt, spec_predicate, update_stmt};
use crate::parse::grammar::specs::quants::{choose_expr, exists_expr, forall_expr, is_at_quant_kw};
use crate::parse::grammar::specs::schemas::{
    apply_schema, global_variable, include_schema, schema_field,
};
use crate::parse::grammar::utils::{delimited_items_with_recover, list};
use crate::parse::grammar::{attributes, error_block, name_ref, patterns, type_args, types};
use crate::parse::parser::{CompletedMarker, Marker, Parser};
use crate::parse::token_set::TokenSet;
use crate::SyntaxKind::*;
use crate::{ts, SyntaxKind, T};

pub(crate) mod atom;

pub(crate) fn expr(p: &mut Parser) -> bool {
    let r = Restrictions {
        forbid_structs: false,
        prefer_stmt: false,
    };
    expr_bp(p, None, r, 1).is_some()
}

// Parses expression with binding power of at least bp.
pub(crate) fn expr_bp(
    p: &mut Parser,
    stmt_m: Option<Marker>,
    mut r: Restrictions,
    bp: u8,
) -> Option<(CompletedMarker, BlockLike)> {
    let stmt_m = stmt_m.unwrap_or_else(|| p.start());
    let mut lhs = match lhs(p, r) {
        Some((lhs, blocklike)) => {
            let lhs = lhs.extend_to(p, stmt_m);
            if r.prefer_stmt && blocklike.is_block() {
                return Some((lhs, BlockLike::Block));
            }
            lhs
        }
        None => {
            stmt_m.abandon(p);
            return None;
        }
    };

    loop {
        let is_range = p.at(T![..]) /*|| p.at(T![..=])*/;
        let (op_bp, op) = current_op(p);
        if op_bp < bp {
            break;
        }
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

        r = Restrictions { prefer_stmt: false, ..r };

        if is_range {
            let has_trailing_expression = p.at_ts(EXPR_FIRST) && !(r.forbid_structs && p.at(T!['{']));
            if !has_trailing_expression {
                // no RHS
                lhs = m.complete(p, RANGE_EXPR);
                break;
            }
        }

        expr_bp(p, None, Restrictions { prefer_stmt: false, ..r }, op_bp + 1);
        lhs = m.complete(p, if is_range { RANGE_EXPR } else { BIN_EXPR });
    }
    Some((lhs, BlockLike::NotBlock))
}

fn is_expr(p: &mut Parser, lhs: CompletedMarker) -> CompletedMarker {
    let m = lhs.precede(p);
    p.bump_remap(T![is]);
    types::type_(p);
    while p.eat(T![|]) {
        types::type_(p);
    }
    m.complete(p, IS_EXPR)
}

fn cast_expr(p: &mut Parser, lhs: CompletedMarker) -> CompletedMarker {
    assert!(p.at(T![as]));
    let m = lhs.precede(p);
    p.bump(T![as]);
    types::type_(p);
    m.complete(p, CAST_EXPR)
}

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
                if p.nth_at(1, T![:]) {
                    name_ref(p);
                    p.expect(T![:]);
                }
                expr(p);
                m.complete(p, STRUCT_LIT_FIELD);
            }
            T!['{'] => {
                error_block(p, "expected a field");
                m.abandon(p);
            }
            _ => {
                p.error_and_bump_any("expected identifier");
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
        T![|] => {
            m = p.start();
            if !lambda_param_list(p) {
                m.abandon(p);
                return None;
            }
            expr_bp(p, None, r, 1);
            let cm = m.complete(p, LAMBDA_EXPR);
            return Some((cm, BlockLike::NotBlock));
        }
        T![&] => {
            m = p.start();
            p.bump(T![&]);
            p.eat(T![mut]);
            BORROW_EXPR
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
        T![..] => {
            m = p.start();
            p.bump(T![..]);
            if p.at_ts(EXPR_FIRST) && !(r.forbid_structs && p.at(T!['{'])) {
                expr_bp(p, None, r, 2);
            }
            let cm = m.complete(p, RANGE_EXPR);
            return Some((cm, BlockLike::NotBlock));
        }
        _ => {
            let (lhs, blocklike) = atom::atom_expr(p)?;

            let allow_calls = !(r.prefer_stmt && blocklike.is_block());
            let cm = postfix_expr(p, lhs, blocklike, allow_calls);

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
) -> (CompletedMarker, BlockLike) {
    loop {
        lhs = match p.current() {
            T!['('] if allow_calls => call_expr(p, lhs),
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

fn postfix_dot_expr(
    p: &mut Parser<'_>,
    lhs: CompletedMarker,
) -> Result<CompletedMarker, CompletedMarker> {
    assert!(p.at(T![.]));

    if p.nth_at(1, IDENT) && (p.nth_at(2, T!['(']) || p.nth_at(2, T![::])) {
        return Ok(method_call_expr(p, lhs));
    }

    dot_expr(p, lhs)
}

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

fn dot_expr(p: &mut Parser<'_>, lhs: CompletedMarker) -> Result<CompletedMarker, CompletedMarker> {
    assert!(p.at(T![.]));
    let m = lhs.precede(p);
    p.bump(T![.]);
    {
        let m = p.start();
        if p.at(IDENT) {
            p.bump(IDENT);
            m.complete(p, NAME_REF);
        } else if p.at(INT_NUMBER) {
            p.bump(INT_NUMBER);
            m.complete(p, NAME_REF);
        } else {
            p.error("expected field name or number");
            m.abandon(p);
        }
    }
    Ok(m.complete(p, DOT_EXPR))
}

fn index_expr(p: &mut Parser<'_>, lhs: CompletedMarker) -> CompletedMarker {
    assert!(p.at(T!['[']));
    let m = lhs.precede(p);
    p.bump(T!['[']);
    expr(p);
    p.expect(T![']']);
    m.complete(p, INDEX_EXPR)
}

fn arg_list(p: &mut Parser<'_>) {
    assert!(p.at(T!['(']));
    let m = p.start();
    p.bump(T!['(']);
    delimited_items_with_recover(p, T![')'], T![,], ts!(T![;], T![let], T!['}']), VALUE_ARG, |p| {
        let m = p.start();
        let is_expr = expr(p);
        if is_expr {
            m.complete(p, VALUE_ARG);
        } else {
            m.abandon(p);
        }
        is_expr
    });
    p.expect(T![')']);
    m.complete(p, VALUE_ARG_LIST);
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
            BLOCK_EXPR | IF_EXPR | WHILE_EXPR | FOR_EXPR | LOOP_EXPR | MATCH_EXPR
        )
    }
}

pub(super) fn stmt(p: &mut Parser, prefer_expr: bool, is_spec: bool) {
    let stmt_m = p.start();

    attributes::outer_attrs(p);

    if p.at(T![let]) {
        let_stmt(p, stmt_m);
        return;
    }
    if p.at(T![use]) {
        use_item::use_stmt(p, stmt_m);
        return;
    }

    if is_spec {
        if p.at(T![native]) && p.nth_at(1, T![fun]) || p.at(T![fun]) {
            fun::spec_inline_function(p);
            stmt_m.abandon(p);
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
            stmt_m.abandon(p);
            return;
        }
    }

    if let Some((cm, _)) = stmt_expr(p, Some(stmt_m)) {
        if !(p.at(T!['}']) || (prefer_expr && p.at(EOF))) {
            let m = cm.precede(p);
            p.expect(T![;]);
            m.complete(p, EXPR_STMT);
        }
        return;
    }

    // p.error(&format!("unexpected token {:?}", p.current()));
    p.error_and_bump_any(&format!("unexpected token {:?}", p.current()));
}

fn let_stmt(p: &mut Parser, m: Marker) {
    p.bump(T![let]);
    if p.at_contextual_kw_ident("post") {
        p.bump_remap(T![post]);
    }
    pattern(p);
    if p.at(T![:]) {
        types::ascription(p);
    }
    opt_initializer_expr(p);
    p.expect(T![;]);

    m.complete(p, LET_STMT);
}

pub(crate) fn opt_initializer_expr(p: &mut Parser) {
    if p.eat(T![=]) {
        if !expr(p) {
            p.error("expected expression");
        }
    }
}

pub(super) fn stmt_expr(p: &mut Parser, stmt_m: Option<Marker>) -> Option<(CompletedMarker, BlockLike)> {
    let r = Restrictions {
        forbid_structs: false,
        prefer_stmt: true,
    };
    expr_bp(p, stmt_m, r, 1)
}

pub(super) fn expr_block_contents(p: &mut Parser, is_spec: bool) {
    while !p.at(EOF) && !p.at(T!['}']) {
        if p.at(T![;]) {
            p.bump(T![;]);
            continue;
        }
        stmt(p, false, is_spec);
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) struct Restrictions {
    pub forbid_structs: bool,
    pub prefer_stmt: bool,
}

/// Binding powers of operators for a Pratt parser.
///
/// See <https://matklad.github.io/2020/04/13/simple-but-powerful-pratt-parsing.html>
#[rustfmt::skip]
fn current_op(p: &Parser) -> (u8, SyntaxKind) {
    const NOT_AN_OP: (u8, SyntaxKind) = (0, T![@]);
    match p.current() {
        T![as]                 => (1, T![as]),
        T![ident] if p.at_contextual_kw("is") => (1, T![is]),

        T![=]                  => (2,  T![=]),
        T![>] if p.at(T![>>=]) => (2,  T![>>=]),
        T![<] if p.at(T![<==>])  => (2, T![<==>]),
        T![<] if p.at(T![<<=]) => (2,  T![<<=]),
        T![==>]  => (2, T![==>]),
        T![+=]  => (2,  T![+=]),
        T![-=]  => (2,  T![-=]),
        T![*=]  => (2,  T![*=]),
        T![/=]  => (2,  T![/=]),
        T![|=]  => (2,  T![|=]),
        T![^=]  => (2,  T![^=]),
        T![&=]  => (2,  T![&=]),
        T![%=]  => (2,  T![%=]),

        T![..]  => (3,  T![..]),

        T![|] if p.at(T![||])  => (4,  T![||]),

        T![&] if p.at(T![&&])  => (5,  T![&&]),

        T![>] if p.at(T![>=])  => (6,  T![>=]),
        T![>] if p.at(T![>>])  => (10,  T![>>]),
        T![>]                  => (6,  T![>]),

        T![<] if p.at(T![<=])  => (6,  T![<=]),
        T![<] if p.at(T![<<])  => (10,  T![<<]),
        T![<]                 => (6,  T![<]),
        T![!=]  => (6,  T![!=]),
        T![==]  => (6,  T![==]),

        T![|]                  => (7,  T![|]),

        T![^]                  => (8,  T![^]),

        T![&]                  => (9,  T![&]),


        T![+]                  => (11, T![+]),
        T![-]                  => (11, T![-]),

        T![%]                  => (12, T![%]),
        T![/]                  => (12, T![/]),
        T![*]                  => (12, T![*]),

        // T![as]                 => (13, T![as]),
        // T![ident] if p.at_contextual_kw("is") => (13, T![is]),

        T![=] if p.at(T![=>])  => NOT_AN_OP,
        _                      => NOT_AN_OP
    }
}
