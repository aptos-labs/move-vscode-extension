use crate::completions::Completions;
use crate::context::CompletionContext;
use std::cell::RefCell;
use std::collections::HashSet;
use std::ops::Sub;

/// The kind of item list a [`PathKind::Item`] belongs to.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ItemListKind {
    SourceFile,
    Module,
    Function { existing_modifiers: HashSet<String> },
}

pub(crate) fn complete_item_list(
    acc: &RefCell<Completions>,
    ctx: &CompletionContext,
    kind: &ItemListKind,
) {
    let _p = tracing::info_span!("complete_item_list", ?kind).entered();

    add_keywords(acc, ctx, Some(kind));
}

fn add_keywords(
    acc: &RefCell<Completions>,
    ctx: &CompletionContext,
    kind: Option<&ItemListKind>,
) -> Option<()> {
    let add_keyword = |kw: &str| {
        acc.borrow_mut()
            .add_keyword_snippet(ctx, kw, format!("{} $0", kw).leak())
    };
    let add_keyword_s = |kw: String| {
        acc.borrow_mut()
            .add_keyword_snippet(ctx, kw.as_str(), format!("{} $0", kw).leak())
    };

    let kind = kind?;

    match kind {
        ItemListKind::SourceFile => {
            add_keyword("module");
            add_keyword("script");
            add_keyword("spec");
        }
        ItemListKind::Module => {
            add_keyword("use");
            add_keyword("fun");
            add_keyword("struct");
            add_keyword("const");
            add_keyword("enum");
            add_keyword("spec");
            add_keyword("friend");

            for function_modifier in all_function_modifiers().into_iter() {
                if function_modifier == "friend" {
                    continue;
                }
                add_keyword_s(function_modifier);
            }
        }
        ItemListKind::Function { existing_modifiers } => {
            let remaining_modifiers = all_function_modifiers().sub(existing_modifiers);
            for modifier in remaining_modifiers {
                add_keyword_s(modifier);
            }
            add_keyword("fun");
        }
    }

    Some(())
}

fn all_function_modifiers() -> HashSet<String> {
    vec!["public", "native", "entry", "inline", "package", "friend"]
        .into_iter()
        .map(|it| it.to_string())
        .collect()
}
