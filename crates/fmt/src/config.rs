use crate::engine::fmt_model::IndentType;

const DEFAULT_MAX_WIDTH: usize = 90;
const DEFAULT_INDENT_SIZE: usize = 4;

// Matches the original formatter's threshold for breaking binary expression chains.
const BIN_EXPR_CHAIN_MIN_LENGTH: usize = 64;

// Mirrors the original formatter's MAX_ANALYZE_LENGTH — reserves ~25% of line
// width for return type and specifiers.
const MAX_ANALYZE_LENGTH: usize = 64;

// Minimum line length before the formatter considers breaking at the return type colon.
const MIN_BREAK_LENGTH: usize = 32;

#[derive(Clone)]
pub struct CstFormatConfig {
    max_width: usize,
    indent_size: usize,
}

impl CstFormatConfig {
    pub fn with_max_width(mut self, max_width: usize) -> Self {
        self.max_width = max_width;
        self
    }

    pub fn block_indent(&self) -> usize {
        self.indent_size
    }

    pub fn continuation_indent(&self) -> usize {
        self.indent_size
    }

    pub fn indent(&self, indent_type: IndentType) -> usize {
        match indent_type {
            IndentType::None => 0,
            IndentType::Continuation => self.indent_size,
            IndentType::Block => self.indent_size,
        }
    }

    pub fn max_line_width(&self) -> usize {
        self.max_width
    }

    pub fn bin_expr_chain_density_limit(&self) -> usize {
        BIN_EXPR_CHAIN_MIN_LENGTH
    }

    pub fn fun_signature_density_limit(&self) -> usize {
        MAX_ANALYZE_LENGTH
    }

    pub fn min_break_length(&self) -> usize {
        MIN_BREAK_LENGTH
    }
}

impl Default for CstFormatConfig {
    fn default() -> Self {
        Self {
            max_width: DEFAULT_MAX_WIDTH,
            indent_size: DEFAULT_INDENT_SIZE,
        }
    }
}
