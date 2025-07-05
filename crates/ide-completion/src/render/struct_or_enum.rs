use crate::context::CompletionContext;
use crate::item::CompletionItemBuilder;
use crate::render::render_named_item;
use lang::types::has_type_params_ext::GenericItemExt;
use syntax::ast;
use syntax::files::InFile;

pub(crate) fn render_struct_or_enum(
    ctx: &CompletionContext<'_>,
    item_name: String,
    struct_or_enum: InFile<ast::StructOrEnum>,
) -> CompletionItemBuilder {
    let mut item_builder = render_named_item(ctx, &item_name, struct_or_enum.clone().value.into());

    let has_type_params = !struct_or_enum.ty_type_params().is_empty();
    let snippet = if has_type_params {
        format!("{item_name}<$0>")
    } else {
        format!("{item_name}$0")
    };
    item_builder.insert_snippet(snippet);

    item_builder
}
