use crate::ast::node_ext::syntax_element::SyntaxElementExt;
use crate::ast::syntax_factory::SyntaxFactory;
use crate::syntax_editor::SyntaxEditor;
use crate::{AstNode, ast};
use itertools::Itertools;
use vfs::FileId;

impl ast::UseStmt {
    pub fn delete(&self, editor: &mut SyntaxEditor) {
        if let Some(following_ws) = self.syntax().following_ws() {
            editor.delete(following_ws);
        }
        editor.delete(self.syntax());
    }

    pub fn delete_group_use_specks(
        &self,
        unused_use_specks: Vec<ast::UseSpeck>,
        editor: &mut SyntaxEditor,
    ) {
        let single_use_speck_left = self
            .group_use_specks()
            .into_iter()
            .filter(|it| !unused_use_specks.contains(it))
            .exactly_one()
            .ok();
        if let Some(single_use_speck_left) = single_use_speck_left {
            // recreate new use stmt
            self.replace_root_use_speck_with(single_use_speck_left, editor);
            return;
        }

        for unused_use_speck in unused_use_specks {
            unused_use_speck.delete(editor);
        }
    }

    fn replace_root_use_speck_with(
        &self,
        use_speck: ast::UseSpeck,
        editor: &mut SyntaxEditor,
    ) -> Option<()> {
        let use_group = self.use_speck()?.use_group()?;
        let root_use_speck = use_group.use_stmt()?.use_speck()?;
        let root_path = root_use_speck.path()?;

        let make = SyntaxFactory::new();
        if use_speck.is_self() {
            editor.replace(
                root_use_speck.syntax(),
                make.use_speck(root_path).syntax().clone_for_update(),
            );
            editor.add_mappings(make.finish_with_mappings());
            return Some(());
        }

        let mut segments = root_path.segments();
        let use_speck_segment = use_speck.path().and_then(|it| it.segment())?;
        segments.push(use_speck_segment);

        editor.replace(
            root_use_speck.syntax(),
            make.use_speck(make.path_from_segments(segments))
                .syntax()
                .clone_for_update(),
        );
        editor.add_mappings(make.finish_with_mappings());
        Some(())
    }
}
