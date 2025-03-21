use crate::context::CompletionContext;
use crate::item::{CompletionItem, CompletionItemBuilder, CompletionItemKind};
use ide_db::SymbolKind;
use stdx::format_to;
use syntax::{ast, AstNode};

pub(crate) fn render_function_completion_item(
    ctx: &CompletionContext<'_>,
    function_name: String,
    function: ast::Fun,
) -> CompletionItemBuilder {
    let function_name = function_name.as_str();

    let mut item = CompletionItem::new(
        CompletionItemKind::SymbolKind(SymbolKind::Function),
        ctx.source_range(),
        function_name,
    );

    let mut detail = String::new();
    if let Some(ret_type) = function.return_type() {
        format_to!(detail, "{}", ret_type.syntax().text());
    }
    item.set_detail(Some(detail));

    if let Some(cap) = ctx.config.snippet_cap {
        let (snippet, label_suffix) = if function.params().is_empty() {
            (format!("{}()$0", function_name), "()")
        } else {
            (format!("{}($0)", function_name), "(..)")
        };
        item.set_label(format!("{}{}", function_name, label_suffix));
        item.insert_snippet(cap, snippet);
    }

    item
}
