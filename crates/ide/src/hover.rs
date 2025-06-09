mod doc_signature;

use crate::RangeInfo;
use crate::hover::doc_signature::DocSignatureOwner;
use ide_db::RootDatabase;
use lang::Semantics;
use std::fmt::Write;
use stdx::itertools::Itertools;
use syntax::algo::find_node_at_offset;
use syntax::ast::HoverDocsOwner;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::{FilePosition, InFileExt};
use syntax::{AstNode, ast};

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
pub(crate) fn hover(
    db: &RootDatabase,
    FilePosition { file_id, offset }: FilePosition,
) -> Option<RangeInfo<HoverResult>> {
    let sema = &Semantics::new(db, file_id);
    let file = sema.parse(file_id).syntax().clone();

    let name_like = find_node_at_offset::<ast::NameLike>(&file, offset)?;
    let name_range = name_like.syntax().text_range();

    let hover_docs_owner = match name_like {
        ast::NameLike::NameRef(name_ref) => {
            let ref_element = name_ref.syntax().ancestor_strict::<ast::ReferenceElement>()?;
            let doc_comments_owner =
                sema.resolve_to_element::<ast::AnyHoverDocsOwner>(ref_element.in_file(file_id))?;
            doc_comments_owner.value
        }
        ast::NameLike::Name(name) => {
            let doc_comments_owner = name.syntax().parent_of_type::<ast::AnyHoverDocsOwner>()?;
            doc_comments_owner
        }
    };

    let named_element = hover_docs_owner.syntax().cast::<ast::NamedElement>()?;

    let ident_token = named_element.name()?.ident_token();
    let doc_comments = hover_docs_owner.outer_doc_comments(ident_token);

    let mut doc_string = String::new();

    writeln!(doc_string, "```move").ok()?;
    named_element.header(sema, &mut doc_string);
    writeln!(doc_string).ok()?;
    named_element.signature(sema, &mut doc_string);
    writeln!(doc_string).ok()?;
    writeln!(doc_string, "```").ok()?;

    // writeln!(doc_string).ok()?;
    // writeln!(doc_string).ok()?;
    writeln!(doc_string, "---").ok()?;
    // writeln!(doc_string).ok()?;
    // writeln!(doc_string).ok()?;

    write!(doc_string, "{}", format_doc_comments(doc_comments)).ok()?;
    writeln!(doc_string,).ok()?;

    Some(RangeInfo::new(name_range, HoverResult { doc_string }))
}

fn format_doc_comments(doc_comments: Vec<ast::Comment>) -> String {
    doc_comments
        .iter()
        .filter_map(|it| it.comment_line())
        .map(|it| it.trim())
        .join("\n")
}
