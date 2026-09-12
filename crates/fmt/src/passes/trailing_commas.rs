use crate::config::CstFormatConfig;
use crate::rules::spacing::ALL_COMMA_SEPARATED_LISTS;
use syntax::ast::node_ext::syntax_element::SyntaxElementExt;
use syntax::syntax_editor::SyntaxEditor;
use syntax::{AstNode, SourceFile, SyntaxNode};

pub fn remove_trailing_commas_in_file(root: SourceFile, config: &CstFormatConfig) -> SourceFile {
    let syntax = root.syntax().clone();
    let mut editor = SyntaxEditor::new(syntax.clone());

    remove_trailing_commas(&mut editor, &syntax);

    let result = editor.finish();
    let new_root = SyntaxNode::new_root(result.new_root().green().into());
    SourceFile::cast(new_root).unwrap()
}

fn remove_trailing_commas(editor: &mut SyntaxEditor, root: &SyntaxNode) {
    let comma_separated_lists = root
        .descendants()
        .filter(|it| ALL_COMMA_SEPARATED_LISTS.contains(it.kind()))
        .collect::<Vec<_>>();

    for list in comma_separated_lists {
        let Some(closing_delimiter) = list.last_child_or_token().and_then(|it| it.into_token()) else {
            continue;
        };

        if let Some(comma) = closing_delimiter.preceding_comma() {
            editor.delete(comma);
        }
    }
}
