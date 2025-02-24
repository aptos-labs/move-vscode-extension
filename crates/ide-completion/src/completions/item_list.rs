use crate::completions::Completions;
use crate::context::CompletionContext;
use std::cell::RefCell;

/// The kind of item list a [`PathKind::Item`] belongs to.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ItemListKind {
    SourceFile,
    Module,
}

pub(crate) fn complete_item_list(
    acc: &RefCell<Completions>,
    ctx: &CompletionContext,
    kind: &ItemListKind,
) {
    let _p = tracing::info_span!("complete_item_list").entered();

    add_keywords(acc, ctx, Some(kind));
}

fn add_keywords(acc: &RefCell<Completions>, ctx: &CompletionContext, kind: Option<&ItemListKind>) {
    let add_keyword_with_shift = |kw| {
        acc.borrow_mut()
            .add_keyword_snippet(ctx, kw, format!("{} $0", kw).leak())
    };

    if matches!(kind, Some(ItemListKind::SourceFile)) {
        add_keyword_with_shift("module");
        add_keyword_with_shift("script");
        add_keyword_with_shift("spec");
    }

    if matches!(kind, Some(ItemListKind::Module)) {
        add_keyword_with_shift("use");
        add_keyword_with_shift("fun");
        add_keyword_with_shift("struct");
        add_keyword_with_shift("const");
        add_keyword_with_shift("enum");
        add_keyword_with_shift("spec");
        add_keyword_with_shift("friend");

        add_keyword_with_shift("public");
        add_keyword_with_shift("native");
        add_keyword_with_shift("entry");
        add_keyword_with_shift("inline");
    }
}
