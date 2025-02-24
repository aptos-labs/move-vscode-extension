use crate::{ast, AstToken, TextRange, TextSize};
use stdx::always;

impl ast::Whitespace {
    pub fn spans_multiple_lines(&self) -> bool {
        let text = self.text();
        text.find('\n').is_some_and(|idx| text[idx + 1..].contains('\n'))
    }
}

#[derive(Debug)]
pub struct QuoteOffsets {
    pub quotes: (TextRange, TextRange),
    pub contents: TextRange,
}

impl QuoteOffsets {
    fn new(literal: &str, left_quote: &str) -> Option<QuoteOffsets> {
        let left_quote_idx = literal.find(left_quote)?;
        let right_quote_idx = literal.rfind('"')?;
        if left_quote_idx == right_quote_idx - (left_quote.len() - 1) {
            // `literal` only contains one quote
            return None;
        }

        let start = TextSize::from(0);
        let left_quote = TextSize::try_from(left_quote_idx).unwrap() + TextSize::of(left_quote);
        let right_quote = TextSize::try_from(right_quote_idx).unwrap();
        let end = TextSize::of(literal);

        let res = QuoteOffsets {
            quotes: (
                TextRange::new(start, left_quote),
                TextRange::new(right_quote, end),
            ),
            contents: TextRange::new(left_quote, right_quote),
        };
        Some(res)
    }
}

pub trait IsString: AstToken {
    const LEFT_QUOTE: &'static str;

    fn quote_offsets(&self) -> Option<QuoteOffsets> {
        let text = self.text();
        let offsets = QuoteOffsets::new(text, Self::LEFT_QUOTE)?;
        let o = self.syntax().text_range().start();
        let offsets = QuoteOffsets {
            quotes: (offsets.quotes.0 + o, offsets.quotes.1 + o),
            contents: offsets.contents + o,
        };
        Some(offsets)
    }

    fn text_range_between_quotes(&self) -> Option<TextRange> {
        self.quote_offsets().map(|it| it.contents)
    }

    fn text_without_quotes(&self) -> &str {
        let text = self.text();
        let Some(offsets) = self.text_range_between_quotes() else {
            return text;
        };
        &text[offsets - self.syntax().text_range().start()]
    }

    fn open_quote_text_range(&self) -> Option<TextRange> {
        self.quote_offsets().map(|it| it.quotes.0)
    }

    fn close_quote_text_range(&self) -> Option<TextRange> {
        self.quote_offsets().map(|it| it.quotes.1)
    }

    fn map_range_up(&self, range: TextRange) -> Option<TextRange> {
        let contents_range = self.text_range_between_quotes()?;
        if always!(TextRange::up_to(contents_range.len()).contains_range(range)) {
            Some(range + contents_range.start())
        } else {
            None
        }
    }
}

impl IsString for ast::ByteString {
    const LEFT_QUOTE: &'static str = "b\"";
}

impl IsString for ast::HexString {
    const LEFT_QUOTE: &'static str = "x\"";
}
