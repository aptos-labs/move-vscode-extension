use crate::completions::Completions;
use crate::context::CompletionContext;
use crate::item::{CompletionItem, CompletionItemKind};
use ide_db::SymbolKind;
use lang::nameres::labels::get_loop_labels_resolve_variants;
use std::cell::RefCell;
use syntax::files::InFileExt;
use syntax::{TextRange, ast};

pub(crate) fn add_label_completions(
    completions: &RefCell<Completions>,
    ctx: &CompletionContext<'_>,
    fake_label: ast::Label,
    source_range: TextRange,
) -> Option<()> {
    let fake_label = fake_label.in_file(ctx.position.file_id);
    let label_entries = get_loop_labels_resolve_variants(fake_label);

    let acc = &mut completions.borrow_mut();

    for label_entry in label_entries {
        let item_builder = CompletionItem::new(
            CompletionItemKind::SymbolKind(SymbolKind::Label),
            source_range,
            label_entry.name,
        );
        acc.add(item_builder.build(ctx.db));
    }

    Some(())
}
