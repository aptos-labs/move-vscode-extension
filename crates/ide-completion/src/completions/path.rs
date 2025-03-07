use crate::completions::Completions;
use crate::context::CompletionContext;
use crate::item::{CompletionItem, CompletionItemKind};
use crate::render::function::render_function_completion_item;
use ide_db::SymbolKind;
use lang::nameres::path_kind::path_kind;
use lang::nameres::paths::{process_path_resolve_variants, PathResolutionContext};
use lang::nameres::processors::collect_entries;
use std::cell::RefCell;
use syntax::{ast, AstNode, SyntaxKind};

pub(crate) fn add_path_completions(
    completions: &RefCell<Completions>,
    ctx: &CompletionContext<'_>,
    path: ast::Path,
) {
    let Some(path_kind) = path_kind(path.clone(), true) else {
        return;
    };

    {
        let acc = &mut completions.borrow_mut();

        let resolution_ctx = PathResolutionContext {
            path: path.clone(),
            is_completion: true,
        };
        let entries = collect_entries(|collector| {
            process_path_resolve_variants(resolution_ctx, path_kind.clone(), collector);
        });
        for entry in entries {
            let entry_name = entry.name;

            if let Some(function) = ast::Fun::cast(entry.named_node.clone()) {
                let completion_item =
                    render_function_completion_item(&ctx, entry_name, function).build(&ctx.db);
                acc.add(completion_item);
                continue;
            }

            let kind = item_to_kind(entry.named_node.kind());
            let completion_item = CompletionItem::new(kind, ctx.source_range(), entry_name.as_str());
            acc.add(completion_item.build(&ctx.db));
        }
    }

    if path_kind.is_unqualified() {
        add_keywords(completions, ctx);
    }
}

fn add_keywords(completions: &RefCell<Completions>, ctx: &CompletionContext<'_>) {
    let add_keyword = |kw| completions.borrow_mut().add_keyword(ctx, kw);
    let add_keyword_with_shift = |kw| {
        completions
            .borrow_mut()
            .add_keyword_snippet(ctx, kw, &format!("{} $0", kw))
    };
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
