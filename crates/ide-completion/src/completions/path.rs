use crate::completions::Completions;
use crate::context::CompletionContext;
use crate::item::{CompletionItem, CompletionItemKind};
use crate::render::function::render_function_completion_item;
use ide_db::SymbolKind;
use std::cell::RefCell;
use syntax::{ast, AstNode, SyntaxKind};

pub(crate) fn add_path_completions(
    completions: &RefCell<Completions>,
    ctx: &CompletionContext<'_>,
    path: ast::Path,
) {
    {
        let acc = &mut completions.borrow_mut();

        let entries = lang::nameres::paths::collect_paths_for_completion(path);
        for entry in entries {
            let entry_name = entry.name;

            if let Some(function) = ast::Fun::cast(entry.syntax.clone()) {
                let completion_item =
                    render_function_completion_item(&ctx, entry_name, function).build(&ctx.db);
                acc.add(completion_item);
                continue;
            }

            let kind = item_to_kind(entry.syntax.kind());
            let completion_item = CompletionItem::new(kind, ctx.source_range(), entry_name.as_str());
            acc.add(completion_item.build(&ctx.db));
        }
    }

    let add_keyword = |kw| completions.borrow_mut().add_keyword(ctx, kw);
    let add_keyword_with_shift = |kw| {
        completions
            .borrow_mut()
            .add_keyword_snippet(ctx, kw, &format!("{} $0", kw))
    };

    // add keywords
    add_keyword_with_shift("if");
    add_keyword_with_shift("match");
    add_keyword_with_shift("loop");
    add_keyword_with_shift("while");
    add_keyword_with_shift("for");

    add_keyword_with_shift("let");

    add_keyword("true");
    add_keyword("false");
}

fn item_to_kind(kind: SyntaxKind) -> CompletionItemKind {
    use syntax::SyntaxKind::*;
    match kind {
        ATTR => CompletionItemKind::SymbolKind(SymbolKind::Attribute),
        FUN => CompletionItemKind::SymbolKind(SymbolKind::Function),
        CONST => CompletionItemKind::SymbolKind(SymbolKind::Const),
        STRUCT => CompletionItemKind::SymbolKind(SymbolKind::Struct),
        ENUM => CompletionItemKind::SymbolKind(SymbolKind::Enum),
        IDENT_PAT => CompletionItemKind::SymbolKind(SymbolKind::Local),
        LABEL => CompletionItemKind::SymbolKind(SymbolKind::Label),
        TYPE_PARAM => CompletionItemKind::SymbolKind(SymbolKind::TypeParam),
        _ => CompletionItemKind::UnresolvedReference,
    }
}
