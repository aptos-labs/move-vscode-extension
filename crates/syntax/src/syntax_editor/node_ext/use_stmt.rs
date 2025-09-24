// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ast::edit::AstNodeEdit;
use crate::ast::node_ext::syntax_element::SyntaxElementExt;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::ast::node_ext::use_speck::UseSpeckKind;
use crate::ast::syntax_factory::SyntaxFactory;
use crate::syntax_editor::{Position, SyntaxEditor};
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

    pub fn add_attribute(&self, editor: &mut SyntaxEditor, attr_name: &str) {
        let make = SyntaxFactory::new();
        let attr = make.attr(attr_name);
        let indent_level = self.indent_level();
        if let Some(use_speck) = self.use_speck() {
            editor.replace(
                self.syntax(),
                make.use_stmt(vec![attr], use_speck.clone_for_update())
                    .indent_inner(indent_level)
                    .syntax(),
            );
        }
    }

    pub fn simplify_root_self(&self, editor: &mut SyntaxEditor) -> Option<()> {
        let root_use_speck = self.use_speck()?;
        // cannot be used with groups
        // if root_use_speck.use_group().is_some() {
        //     return None;
        // }
        if root_use_speck.is_root_self() {
            let module_path = root_use_speck.path()?.qualifier()?;
            let make = SyntaxFactory::new();
            let alias = root_use_speck.use_alias().map(|it| it.clone_for_update());
            editor.replace(
                root_use_speck.syntax(),
                make.root_use_speck(module_path.clone_for_update(), None, alias)
                    .syntax(),
            );
        }
        Some(())
    }

    pub fn add_group_item(
        &self,
        editor: &mut SyntaxEditor,
        name_ref_with_alias: (ast::NameRef, Option<ast::UseAlias>),
    ) -> Option<()> {
        let root_use_speck = self.use_speck()?;
        let root_use_speck_kind = root_use_speck.kind()?;
        let make = SyntaxFactory::new();
        let new_use_speck = match root_use_speck_kind {
            UseSpeckKind::Module { path, alias } => make
                .use_speck_with_group(path, vec![(make.name_ref("Self"), alias), name_ref_with_alias]),
            UseSpeckKind::Item {
                module_path,
                item_name_ref,
                alias,
            } => {
                let mut name_refs = vec![];
                if let Some(item_name_ref) = item_name_ref {
                    name_refs.push((item_name_ref, alias));
                }
                name_refs.push(name_ref_with_alias);
                make.use_speck_with_group(module_path, name_refs)
            }
            UseSpeckKind::Group { module_path, mut name_refs } => {
                name_refs.push(name_ref_with_alias);
                make.use_speck_with_group(module_path, name_refs)
            }
            _ => {
                return None;
            }
        };
        editor.replace(root_use_speck.syntax(), new_use_speck.syntax());
        editor.add_mappings(make.finish_with_mappings());
        Some(())
    }

    pub fn delete_group_use_specks(
        &self,
        editor: &mut SyntaxEditor,
        unused_use_specks: Vec<ast::UseSpeck>,
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
        let new_root_path = if use_speck.is_group_self() {
            root_path
        } else {
            let mut segments = root_path.segments();
            let use_speck_segment = use_speck.path().and_then(|it| it.segment())?;
            segments.push(use_speck_segment);
            make.path_from_segments(segments)
        };

        let alias = use_speck.use_alias().map(|it| it.clone_for_update());
        editor.replace(
            root_use_speck.syntax(),
            make.use_speck(new_root_path, alias).syntax(),
        );
        editor.add_mappings(make.finish_with_mappings());
        Some(())
    }
}
