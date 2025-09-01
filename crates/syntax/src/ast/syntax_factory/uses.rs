use crate::ast::syntax_factory::{SyntaxFactory, module_item_from_text};
use crate::{AstNode, ast};
use itertools::Itertools;

impl SyntaxFactory {
    pub fn use_stmt(
        &self,
        attrs: impl IntoIterator<Item = ast::Attr>,
        use_speck: ast::UseSpeck,
    ) -> ast::UseStmt {
        #[rustfmt::skip]
        let attrs =
            attrs.into_iter().fold(String::new(), |mut acc, attr| stdx::format_to_acc!(acc, "{}\n", attr));
        module_item_from_text::<ast::UseStmt>(&format!("{attrs}use {use_speck};")).clone_for_update()
    }

    pub fn use_speck(&self, path: ast::Path, alias: Option<ast::UseAlias>) -> ast::UseSpeck {
        let mut buf = "use ".to_string();
        buf += &path.syntax().to_string();
        if let Some(alias) = alias {
            stdx::format_to!(buf, " {alias}");
        }
        module_item_from_text::<ast::UseSpeck>(&buf).clone_for_update()
    }

    pub fn use_speck_with_group(
        &self,
        module_path: ast::Path,
        name_refs: Vec<(ast::NameRef, Option<ast::UseAlias>)>,
    ) -> ast::UseSpeck {
        let mut buf = "use ".to_string();
        let buf = format!(
            "use {}::{{{}}}",
            module_path.syntax().text(),
            name_refs
                .iter()
                .map(|(name_ref, alias)| {
                    let mut item_text = name_ref.text().to_string();
                    if let Some(alias_name) = alias.clone().and_then(|it| it.name()) {
                        stdx::format_to!(item_text, " as {alias_name}");
                    }
                    item_text
                })
                .join(", ")
        );
        module_item_from_text::<ast::UseSpeck>(&buf).clone_for_update()
    }
}
