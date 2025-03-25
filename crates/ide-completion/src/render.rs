use crate::context::CompletionContext;
use crate::item::{CompletionItem, CompletionItemBuilder, CompletionItemKind};
use ide_db::SymbolKind;
use syntax::ast::NamedElement;
use syntax::SyntaxKind::{
    ATTR, CONST, ENUM, FUN, IDENT_PAT, LABEL, MODULE, NAMED_FIELD, STRUCT, TYPE_PARAM,
};
use syntax::{ast, AstNode, SyntaxKind};

pub(crate) mod function;

pub(crate) fn render_named_item(
    ctx: &CompletionContext<'_>,
    named_item: ast::AnyNamedElement,
) -> CompletionItemBuilder {
    let item_name = named_item.name().expect("handled on upper level").as_string();
    let completion_kind = item_to_kind(named_item.syntax().kind());
    let mut completion_item = CompletionItem::new(completion_kind, ctx.source_range(), &item_name);

    // if let Some(function) = named_item.cast_into::<ast::Fun>() {
    //     let ret_type = function.return_type().map(|it| it.syntax().text());
    //     completion_item.set_detail(ret_type);
    //
    //     if let Some(cap) = ctx.config.snippet_cap {
    //         let (snippet, label_suffix) = if function.params().is_empty() {
    //             (format!("{}()$0", &item_name), "()")
    //         } else {
    //             (format!("{}($0)", &item_name), "(..)")
    //         };
    //         completion_item.set_label(format!("{}{}", &item_name, label_suffix));
    //         completion_item.insert_snippet(cap, snippet);
    //     }
    // }

    completion_item
}

fn item_to_kind(kind: SyntaxKind) -> CompletionItemKind {
    use syntax::SyntaxKind::*;
    match kind {
        MODULE => CompletionItemKind::SymbolKind(SymbolKind::Module),
        ATTR => CompletionItemKind::SymbolKind(SymbolKind::Attribute),
        FUN => CompletionItemKind::SymbolKind(SymbolKind::Function),
        CONST => CompletionItemKind::SymbolKind(SymbolKind::Const),
        STRUCT => CompletionItemKind::SymbolKind(SymbolKind::Struct),
        ENUM => CompletionItemKind::SymbolKind(SymbolKind::Enum),
        IDENT_PAT => CompletionItemKind::SymbolKind(SymbolKind::Local),
        LABEL => CompletionItemKind::SymbolKind(SymbolKind::Label),
        TYPE_PARAM => CompletionItemKind::SymbolKind(SymbolKind::TypeParam),
        NAMED_FIELD => CompletionItemKind::SymbolKind(SymbolKind::Field),
        _ => {
            tracing::info!("Unhandled completion item {:?}", kind);
            CompletionItemKind::UnresolvedReference
        }
    }
}
