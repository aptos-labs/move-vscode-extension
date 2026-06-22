use crate::SyntaxKind::{BEHAVIOR_PREDICATE_EXPR, PATH};
use crate::T;
use crate::parse::grammar::expressions::{BlockLike, value_arg_list};
use crate::parse::grammar::paths;
use crate::parse::grammar::paths::{PathMode, path_segment};
use crate::parse::grammar::type_args::opt_type_arg_list_for_expr;
use crate::parse::parser::{CompletedMarker, Parser};

pub(crate) fn behavior_predicate(p: &mut Parser) -> Option<CompletedMarker> {
    if (p.at_contextual_kw_ident("requires_of")
        || p.at_contextual_kw_ident("aborts_of")
        || p.at_contextual_kw_ident("ensures_of")
        || p.at_contextual_kw_ident("result_of"))
        && p.nth_at(1, T![<])
    {
        let m = p.start();
        p.bump(T![ident]);
        opt_type_arg_list_for_expr(p, false);
        value_arg_list(p);
        let cm = m.complete(p, BEHAVIOR_PREDICATE_EXPR);
        return Some(cm);
    }
    None
}
