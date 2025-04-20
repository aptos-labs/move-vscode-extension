use crate::context::CompletionContext;
use crate::item::{CompletionItem, CompletionItemBuilder, CompletionItemKind};
use ide_db::RootDatabase;

pub mod item_list;
pub mod reference;

/// Represents an in-progress set of completions being built.
#[derive(Debug, Default)]
pub struct Completions {
    buf: Vec<CompletionItem>,
}

impl From<Completions> for Vec<CompletionItem> {
    fn from(val: Completions) -> Self {
        val.buf
    }
}

impl CompletionItemBuilder {
    /// Convenience method, which allows to add a freshly created completion into accumulator
    /// without binding it to the variable.
    pub(crate) fn add_to(self, acc: &mut Completions, db: &RootDatabase) {
        acc.add(self.build(db))
    }
}

impl Completions {
    pub(crate) fn add(&mut self, item: CompletionItem) {
        self.buf.push(item)
    }

    fn add_opt(&mut self, item: Option<CompletionItem>) {
        if let Some(item) = item {
            self.buf.push(item)
        }
    }

    pub(crate) fn add_keyword(&mut self, ctx: &CompletionContext, kw: &'static str) {
        let item = CompletionItem::new(CompletionItemKind::Keyword, ctx.source_range(), kw);
        item.add_to(self, ctx.db);
    }

    pub(crate) fn add_keyword_snippet(&mut self, ctx: &CompletionContext<'_>, kw: &str, snippet: &str) {
        let mut item = CompletionItem::new(CompletionItemKind::Keyword, ctx.source_range(), kw);

        match ctx.config.snippet_cap {
            Some(cap) => item.insert_snippet(cap, snippet),
            None => item.insert_text(if snippet.contains('$') { kw } else { snippet }),
        };
        item.add_to(self, ctx.db);
    }
}
