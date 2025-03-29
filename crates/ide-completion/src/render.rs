use crate::context::CompletionContext;
use crate::item::{CompletionItem, CompletionItemBuilder, CompletionItemKind};
use ide_db::SymbolKind;
use syntax::ast::NamedElement;
use syntax::files::InFile;
use syntax::{AstNode, SyntaxKind, ast};

pub(crate) mod function;

pub(crate) fn render_named_item(
    ctx: &CompletionContext<'_>,
    named_item: InFile<ast::AnyNamedElement>,
) -> CompletionItemBuilder {
    let (_, named_item) = named_item.unpack();
    let item_name = named_item.name().expect("handled on upper level").as_string();
    let completion_kind = item_to_kind(named_item.syntax().kind());
    CompletionItem::new(completion_kind, ctx.source_range(), &item_name)
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
