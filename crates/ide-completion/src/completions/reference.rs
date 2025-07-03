mod labels;
mod method_or_field;
pub(crate) mod paths;

use crate::completions::Completions;
use crate::completions::reference::labels::add_label_completions;
use crate::completions::reference::method_or_field::add_method_or_field_completions;
use crate::completions::reference::paths::add_path_completions;
use crate::context::{CompletionContext, ReferenceKind};
use crate::item::CompletionItemKind;
use crate::render::render_named_item;
use lang::node_ext::item::ModuleItemExt;
use std::cell::RefCell;
use syntax::ast;
use syntax::files::{InFile, InFileExt};

pub(crate) fn add_reference_completions(
    completions: &RefCell<Completions>,
    ctx: &CompletionContext<'_>,
    reference_kind: ReferenceKind,
) -> Option<()> {
    let file_id = ctx.position.file_id;
    match reference_kind {
        ReferenceKind::Path { original_path, fake_path } => add_path_completions(
            completions,
            ctx,
            original_path.map(|it| it.in_file(file_id)),
            fake_path,
        ),
        ReferenceKind::FieldRef { receiver_expr } => {
            add_method_or_field_completions(completions, ctx, receiver_expr.in_file(file_id))
        }
        ReferenceKind::Label { fake_label, source_range } => {
            add_label_completions(completions, ctx, fake_label, source_range)
        }
        ReferenceKind::ItemSpecRef { original_item_spec } => {
            add_item_spec_ref_completions(completions, ctx, original_item_spec.in_file(file_id))
        }
    }
}

fn add_item_spec_ref_completions(
    completions: &RefCell<Completions>,
    ctx: &CompletionContext<'_>,
    item_spec: InFile<ast::ItemSpec>,
) -> Option<()> {
    let acc = &mut completions.borrow_mut();

    acc.add(ctx.new_snippet_item(CompletionItemKind::Keyword, "module $0"));
    acc.add(ctx.new_snippet_item(CompletionItemKind::Keyword, "schema $0"));
    acc.add(ctx.new_snippet_item(CompletionItemKind::Keyword, "fun $0"));

    let module = item_spec.module(ctx.db)?;
    for named_item in module.flat_map(|it| it.verifiable_items()) {
        if let Some(name) = named_item.value.name() {
            let name = name.as_string();
            let mut comp_item = render_named_item(ctx, &name, named_item);
            comp_item.insert_snippet(format!("{name} $0"));
            acc.add(comp_item.build(ctx.db));
        }
    }

    Some(())
}
