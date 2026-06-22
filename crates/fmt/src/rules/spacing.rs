use crate::engine::spacing_rules::{
    SpacingRule, after, after_ts, around, before, before_ts, between, between_ts,
};
use syntax::SyntaxKind::*;
use syntax::parse::token_set::TokenSet;
use syntax::{T, ts};

#[rustfmt::skip]
pub(crate) fn build_spacing_rules() -> Vec<SpacingRule> {
    vec![
        // doc comments
        between_ts(ts!(COMMENT), TOP_LEVEL_ITEMS).line_break(),
        between_ts(ts!(COMMENT), MODULE_ITEMS).line_break(),

        // top-level items at root
        after_ts(TOP_LEVEL_ITEMS).line_break(),

        // closing brace of flat blocks (MODULE, SCRIPT, MODULE_SPEC)
        between(T!['{'], COMMENT)
            .inside_ts(FLAT_BRACE_BLOCKS)
            .spaces_or_preserve_line_break(1),
        before(T!['}']).inside_ts(FLAT_BRACE_BLOCKS).line_break(),

        before(COMMENT).spaces_or_preserve_line_break(1),
        // comments and attributes precede items — newline after each
        after(COMMENT).line_break(),
        after(ATTR).line_break(),

        after(VISIBILITY_MODIFIER).spaces(1),
        before(T!['(']).inside(VISIBILITY_MODIFIER).spaces(0),
        after(T!['(']).inside(VISIBILITY_MODIFIER).spaces(0),
        before(T![')']).inside(VISIBILITY_MODIFIER).spaces(0),

        before_ts(MODULE_ITEMS).line_break(),
        before_ts(ALL_STMTS).line_break(),
        after_ts(ALL_STMTS).line_break(),

        // Spec predicate properties: `ensures [abstract] <expr>`.
        after(SPEC_PREDICATE_PROPERTY_LIST)
            .inside(SPEC_PREDICATE_STMT)
            .spaces_or_line_break(1),
        before(ABORTS_IF_WITH)
            .inside(ABORTS_IF_STMT)
            .spaces_or_line_break(1),
        before(IMPLIES)
            .inside(IMPLY_INCLUDE_EXPR)
            .line_break(),

        // Struct fields — newline between fields if they don't fit on one line
        after(T![,]).inside(NAMED_FIELD_LIST).line_break(),

        // paren lists
        before(T!['(']).inside_ts(ALL_PARENS_WRAPPED_ITEMS).spaces(0),
        between(T!['('], T![')']).inside_ts(ALL_PARENS_WRAPPED_ITEMS).spaces(0),

        after(T!['(']).inside_ts(ALL_PARENS_WRAPPED_ITEMS).spaces_or_line_break(0),
        after(T![,])
            .inside_ts(ALL_PARENS_WRAPPED_ITEMS)
            .dependent_line_break(1),
        between(VALUE_ARG, T![,])
            .inside(VALUE_ARG_LIST)
            .spaces(0),
        before(T![')'])
            .inside_ts(ALL_PARENS_WRAPPED_ITEMS)
            .dependent_line_break(0),

        // return type: normalize whitespace before RET_TYPE node (between `)` and `:`)
        before(RET_TYPE).spaces(0),

        // blocks

        // standalone block expression statement: `{ ... };`
        before(BLOCK_EXPR).inside(EXPR_STMT).spaces(0),
        before(SPEC_BLOCK_EXPR).inside(EXPR_STMT).spaces(0),
        before(USE_GROUP).spaces(0),

        // { is inside the block, we need space before the whole block
        before_ts(BLOCK_LIKE).spaces(1),

        // collapse empty blocks
        between(T!['{'], T!['}']).inside_ts(BLOCK_LIKE).spaces(0),

        // code blocks
        // always line break code blocks and named field lists
        between(T!['{'], COMMENT)
            .inside(BLOCK_EXPR)
            .spaces_or_preserve_line_break(1),
        after(T!['{']).inside(BLOCK_EXPR).line_break(),
        before(T!['}']).inside(BLOCK_EXPR).line_break(),

        // always add line breaks
        after(T!['{']).inside(NAMED_FIELD_LIST).line_break(),
        before(T!['}']).inside(NAMED_FIELD_LIST).line_break(),

        after(T!['{']).inside(USE_GROUP).spaces_or_line_break(0),
        before(T!['}']).inside(USE_GROUP).dependent_line_break(0),

        after(T!['{']).inside_ts(BLOCK_LIKE).spaces_or_line_break(1),

        before(T!['}']).inside(BLOCK_EXPR).line_break(),
        before(T!['}']).inside_ts(BLOCK_LIKE).dependent_line_break(1),

        after(T![,]).inside_ts(BRACE_DELIMITED_LISTS).dependent_line_break(1),

        // Punctuation
        before(T![;]).spaces(0),
        after(T![:]).spaces(1),
        before(T![:]).spaces(0),

        // `@0x1`
        after(T![@]).inside(ADDRESS_LIT).spaces(0),

        // refs
        between(T![&], T![mut]).spaces(0),
        after(T![&]).inside(REF_TYPE).spaces(0),
        after(T![&]).inside(BORROW_EXPR).spaces(0),

        // keywords
        after_ts(KEYWORDS).spaces(1),

        before(INITIALIZER).inside(CONST).spaces(1),
        before(T![=]).inside(INITIALIZER).spaces(0),
        // assigment
        before(T![=]).spaces(1),
        after(T![=]).spaces_or_line_break(1),

        // *ref
        around(T![*]).inside(DEREF_EXPR).spaces(0),

        // bin expr
        before_ts(BIN_OPS).dependent_line_break(1),
        after_ts(BIN_OPS).spaces(1),

        // split pragmas into a separate lines if too long
        after(T![,]).inside(PRAGMA_STMT).dependent_line_break(1),
    ]
}

const TOP_LEVEL_ITEMS: TokenSet = ts!(ADDRESS_DEF, MODULE, SCRIPT, MODULE_SPEC);
const MODULE_ITEMS: TokenSet = ts!(FUN, STRUCT, ENUM, CONST, USE_STMT, FRIEND, ITEM_SPEC);

pub(crate) const KEYWORDS: TokenSet = ts!(
    T![address],
    T![module],
    T![script],
    T![spec],
    T![fun],
    T![struct],
    T![const],
    T![friend],
    T![use],
    T![public],
    T![native],
    T![if],
    T![else],
    T![true],
    T![false],
    T![loop],
    T![while],
    T![continue],
    T![break],
    T![as],
    T![return],
    T![move],
    T![let],
    T![mut],
    T![abort],
    T![invariant],
    T![acquires],
    T![inline],
    T![entry],
    T![package],
    T![match],
    T![for],
    T![in],
    T![is],
    T![phantom],
    T![enum],
    T![has],
    T![assert],
    T![assume],
    T![requires],
    T![ensures],
    T![aborts_if],
    T![post],
    T![succeeds_if],
    T![aborts_with],
    T![decreases],
    T![modifies],
    T![with],
    T![axiom],
    T![include],
    T![pragma],
    T![global],
    T![local],
    T![update],
    T![copy],
    T![schema],
    T![emits],
    T![apply],
    T![to],
    T![except],
    T![internal],
    T![forall],
    T![exists],
    T![choose],
    T![where],
    T![min],
    T![proof],
    T![lemma],
    T![split],
    T![weight],
);

const ALL_STMTS: TokenSet = MOVE_STMTS.union(SPEC_STMTS).union(PROOF_STMTS);
const MOVE_STMTS: TokenSet = ts!(LET_STMT, EXPR_STMT, USE_STMT);
const SPEC_STMTS: TokenSet = ts!(
    ABORTS_IF_STMT,
    ABORTS_WITH_STMT,
    SPEC_PREDICATE_STMT,
    INVARIANT_STMT,
    EMITS_STMT,
    AXIOM_STMT,
    UPDATE_STMT,
    GLOBAL_VARIABLE_DECL,
    SCHEMA,
    INCLUDE_SCHEMA,
    APPLY_SCHEMA,
    PRAGMA_STMT,
    SPEC_FUN,
    SPEC_INLINE_FUN,
    LEMMA,
);
const PROOF_STMTS: TokenSet = ts!(POST_STMT, SPLIT_STMT);

const BLOCK_LIKE: TokenSet = ts!(
    BLOCK_EXPR,
    SPEC_BLOCK_EXPR,
    NAMED_FIELD_LIST,
    VARIANT_LIST,
    MATCH_ARM_LIST,
    STRUCT_LIT_FIELD_LIST,
    STRUCT_PAT_FIELD_LIST,
    SCHEMA_LIT_FIELD_LIST,
    USE_GROUP,
);
const ALL_PARENS_WRAPPED_ITEMS: TokenSet =
    PAREN_DELIMITED_LISTS.union(ts!(PAREN_EXPR, TUPLE_TYPE, CONDITION,));

pub(crate) const ALL_COMMA_SEPARATED_LISTS: TokenSet = PAREN_DELIMITED_LISTS
    .union(ANGLE_DELIMITED_LISTS)
    .union(BRACE_DELIMITED_LISTS)
    .union(ts!(NAMED_FIELD_LIST));

const PAREN_DELIMITED_LISTS: TokenSet = ts!(
    VALUE_ARG_LIST,
    PARAM_LIST,
    ITEM_SPEC_PARAM_LIST,
    ATTR_ITEM_LIST,
    LAMBDA_PARAM_LIST,
    PAREN_EXPR,
    TUPLE_TYPE,
    TUPLE_EXPR,
    CONDITION,
);
const ANGLE_DELIMITED_LISTS: TokenSet = ts!(TYPE_PARAM_LIST, TYPE_ARG_LIST, ITEM_SPEC_TYPE_PARAM_LIST);
const BRACE_DELIMITED_LISTS: TokenSet = ts!(
    STRUCT_LIT_FIELD_LIST,
    STRUCT_PAT_FIELD_LIST,
    SCHEMA_LIT_FIELD_LIST
);

const FLAT_BRACE_BLOCKS: TokenSet = ts!(MODULE, SCRIPT, MODULE_SPEC, STRUCT_PAT);

pub(crate) const BIN_OPS: TokenSet = ts!(
    PLUS,
    MINUS,
    STAR,
    SLASH,
    PERCENT,
    AMP_AMP,
    PIPE_PIPE,
    EQ_EQ,
    NOT_EQ,
    LT_EQ,
    GT_EQ,
    SHL,
    SHR,
    IMPLIES,
    LESS_IMPLIES
);
