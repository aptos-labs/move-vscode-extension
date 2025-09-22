use lang::item_scope::ItemScope;
use syntax::ast::edit::{AstNodeEdit, IndentLevel};
use syntax::ast::syntax_factory::SyntaxFactory;
use syntax::ast::{HasAttrs, UseStmtsOwner};
use syntax::syntax_editor::SyntaxEditor;
use syntax::{AstNode, ast};

pub fn add_import_for_import_path(
    items_owner: &ast::AnyHasItems,
    import_path: String,
    add_scope: Option<ItemScope>,
) -> impl FnOnce(&mut SyntaxEditor) -> Option<()> {
    let add_test_only = add_scope.is_some_and(|it| it.is_test());
    move |editor| {
        let make = SyntaxFactory::new();
        let (module_path, item_name_ref) = make.fq_path_from_import_path(import_path)?;

        // // try to find existing use stmt for the module path first
        let existing_use_stmt = items_owner
            .use_stmts_for_module_path(&module_path)
            .filter(|it| !it.is_verify_only() && it.is_test_only() == add_test_only)
            .last();

        match existing_use_stmt {
            Some(use_stmt) => {
                let new_group_name_ref = match item_name_ref {
                    Some(item_name_ref) => item_name_ref,
                    None => make.name_ref("Self"),
                };
                use_stmt.add_group_item(editor, (new_group_name_ref, None));
            }
            None => {
                let use_speck_path = match item_name_ref {
                    Some(item_name_ref) => {
                        make.path_from_qualifier_and_name_ref(module_path, item_name_ref)
                    }
                    None => module_path,
                };
                let indent = IndentLevel::from_node(items_owner.syntax()) + 1;
                let attrs = add_test_only.then_some(make.attr("test_only"));
                let use_speck = make.use_speck(use_speck_path, None);
                let new_use_stmt = make.use_stmt(attrs, use_speck).indent_inner(indent);

                items_owner.add_use_stmt(editor, &new_use_stmt);
            }
        }
        editor.add_mappings(make.finish_with_mappings());

        Some(())
    }
}
