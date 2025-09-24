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

    pub fn root_use_speck(
        &self,
        module_path: ast::Path,
        name_ref: Option<ast::NameRef>,
        alias: Option<ast::UseAlias>,
    ) -> ast::UseSpeck {
        let use_speck_path = match name_ref {
            Some(item_name_ref) => self.path_from_qualifier_and_name_ref(module_path, item_name_ref),
            None => module_path,
        };
        self.use_speck(use_speck_path, alias)
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
