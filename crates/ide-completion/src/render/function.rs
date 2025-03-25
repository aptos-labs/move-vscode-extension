use crate::context::CompletionContext;
use crate::item::CompletionItemBuilder;
use crate::render::render_named_item;
use syntax::ast::NamedElement;
use syntax::{ast, AstNode};

pub(crate) fn render_function(
    ctx: &CompletionContext<'_>,
    function: ast::Fun,
    kind: FunctionKind,
) -> CompletionItemBuilder {
    let mut completion_item = render_named_item(ctx, function.clone().into());

    let function_name = function.name().unwrap().as_string();

    let ret_type = function.return_type().map(|it| it.syntax().text());
    completion_item.set_detail(ret_type);

    let params = match kind {
        FunctionKind::Fun => function.params(),
        FunctionKind::Method => function.params().into_iter().skip(1).collect(),
    };
    if let Some(cap) = ctx.config.snippet_cap {
        let (snippet, label_suffix) = if params.is_empty() {
            (format!("{}()$0", &function_name), "()")
        } else {
            (format!("{}($0)", &function_name), "(..)")
        };
        completion_item.set_label(format!("{}{}", &function_name, label_suffix));
        completion_item.insert_snippet(cap, snippet);
    }

    completion_item
}

pub(crate) enum FunctionKind {
    Fun,
    Method,
}
