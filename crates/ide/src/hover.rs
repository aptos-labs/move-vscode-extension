mod item_signature;

use crate::RangeInfo;
use crate::hover::item_signature::DocSignatureOwner;
use base_db::Upcast;
use ide_db::RootDatabase;
use lang::files::InFileExt;
use lang::{FilePosition, Semantics};
use std::fmt::Write;
use stdx::itertools::Itertools;
use syntax::algo::find_node_at_offset;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::ast::{DocCommentsOwner, NamedElement};
use syntax::{AstNode, AstToken, ast};

/// Contains the results when hovering over an item
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub struct HoverResult {
    pub doc_string: String,
    // pub actions: Vec<HoverAction>,
}

// Feature: Hover
//
// Shows additional information, like the type of an expression or the documentation for a definition when "focusing" code.
// Focusing is usually hovering with a mouse, but can also be triggered with a shortcut.
//
// ![Hover](https://user-images.githubusercontent.com/48062697/113020658-b5f98b80-917a-11eb-9f88-3dbc27320c95.gif)
pub(crate) fn hover(db: &RootDatabase, file_position: FilePosition) -> Option<RangeInfo<HoverResult>> {
    let sema = &Semantics::new(db);
    let FilePosition { file_id, offset } = file_position;
    let file = sema.parse(file_id).syntax().clone();

    let name_like = find_node_at_offset::<ast::NameLike>(&file, offset)?;
    let name_range = name_like.syntax().text_range();

    let doc_comment_owner = match name_like {
        ast::NameLike::NameRef(name_ref) => {
            let ref_element = name_ref.syntax().ancestor_strict::<ast::AnyReferenceElement>()?;
            let entry = ref_element.in_file(file_id).resolve(db.upcast())?;
            let doc_comments_owner = entry.cast_into::<ast::AnyDocCommentsOwner>(db.upcast())?;
            doc_comments_owner.value
        }
        ast::NameLike::Name(name) => {
            let doc_comments_owner = name.syntax().parent_of_type::<ast::AnyDocCommentsOwner>()?;
            doc_comments_owner
        }
    };

    let ident_token = doc_comment_owner.name()?.ident_token();
    let doc_comments = doc_comment_owner.outer_doc_comments(ident_token);

    let named_element = ast::AnyNamedElement::cast_from(doc_comment_owner);

    let mut doc_string = String::new();

    writeln!(doc_string, "```").ok()?;

    named_element.owner(&mut doc_string);
    writeln!(doc_string).ok()?;
    named_element.signature(&mut doc_string);

    writeln!(doc_string).ok()?;
    writeln!(doc_string, "```").ok()?;

    writeln!(doc_string).ok()?;
    writeln!(doc_string).ok()?;
    write!(doc_string, "---").ok()?;
    writeln!(doc_string).ok()?;
    writeln!(doc_string).ok()?;

    write!(doc_string, "{}", format_doc_comments(doc_comments)).ok()?;

    Some(RangeInfo::new(name_range, HoverResult { doc_string }))
}

fn format_doc_comments(doc_comments: Vec<ast::Comment>) -> String {
    doc_comments
        .iter()
        .filter_map(|it| it.comment_line())
        .map(|it| it.trim())
        .join("\n")
}
