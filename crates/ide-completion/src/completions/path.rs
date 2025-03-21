use crate::completions::Completions;
use crate::context::CompletionContext;
use crate::item::{CompletionItem, CompletionItemKind};
use crate::render::function::render_function_completion_item;
use base_db::Upcast;
use ide_db::SymbolKind;
use lang::nameres::path_kind::path_kind;
use lang::nameres::paths::{get_path_resolve_variants, ResolutionContext};
use lang::nameres::scope::ScopeEntryListExt;
use lang::InFile;
use std::cell::RefCell;
use syntax::{ast, SyntaxKind};

pub(crate) fn add_path_completions(
    completions: &RefCell<Completions>,
    ctx: &CompletionContext<'_>,
    path: InFile<ast::Path>,
) {
    let path_kind = path_kind(path.clone(), true);
    tracing::debug!(path_kind = ?path_kind);

    if path_kind.is_unqualified() {
        add_keywords(completions, ctx);
    }

    let acc = &mut completions.borrow_mut();

    let resolution_ctx = ResolutionContext {
        path: path.clone(),
        is_completion: true,
    };
    let entries = get_path_resolve_variants(ctx.db.upcast(), &resolution_ctx, path_kind);
    tracing::debug!(entries = ?entries);

    let filtered_entries = entries.filter_by_visibility(ctx.db.upcast(), path.map(|it| it.reference()));
    tracing::debug!(filtered_entries = ?filtered_entries);

    for entry in filtered_entries {
        let entry_name = entry.name;

        if let Some(function) = entry.node_loc.cast::<ast::Fun>(ctx.db.upcast()) {
            let completion_item =
                render_function_completion_item(&ctx, entry_name, function.value).build(&ctx.db);
            acc.add(completion_item);
            continue;
        }

        let kind = item_to_kind(entry.node_loc.kind());
        let completion_item = CompletionItem::new(kind, ctx.source_range(), entry_name.as_str());
        acc.add(completion_item.build(&ctx.db));
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
        MODULE => CompletionItemKind::SymbolKind(SymbolKind::Module),
        ATTR => CompletionItemKind::SymbolKind(SymbolKind::Attribute),
        FUN => CompletionItemKind::SymbolKind(SymbolKind::Function),
        CONST => CompletionItemKind::SymbolKind(SymbolKind::Const),
        STRUCT => CompletionItemKind::SymbolKind(SymbolKind::Struct),
        ENUM => CompletionItemKind::SymbolKind(SymbolKind::Enum),
        IDENT_PAT => CompletionItemKind::SymbolKind(SymbolKind::Local),
        LABEL => CompletionItemKind::SymbolKind(SymbolKind::Label),
        TYPE_PARAM => CompletionItemKind::SymbolKind(SymbolKind::TypeParam),
        _ => {
            tracing::info!("Unhandled completion item {:?}", kind);
            CompletionItemKind::UnresolvedReference
        }
    }
}
