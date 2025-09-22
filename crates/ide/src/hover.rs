// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

mod named_element;
mod spec_keywords;

use crate::RangeInfo;
use crate::hover::named_element::DocSignatureOwner;
use ide_db::RootDatabase;
use lang::Semantics;
use lang::node_ext::item_spec::ItemSpecExt;
use lang::types::ty_db;
use std::fmt::Write;
use stdx::itertools::Itertools;
use syntax::algo::find_node_at_offset;
use syntax::ast::HoverDocsOwner;
use syntax::ast::node_ext::syntax_element::SyntaxElementExt;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::ast::token_at_offset_ext::TokenAtOffsetExt;
use syntax::files::{FilePosition, InFileExt};
use syntax::{AstNode, ast};
use vfs::FileId;

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
    let sema = Semantics::new(db, file_id);
    let file = sema.parse(file_id).syntax().clone();

    if let Some(token) = file.token_at_offset(offset).prefer_no_trivia() {
        if token.kind().is_keyword() {
            return spec_keywords::spec_keyword_docs(db, file_id, token);
        }
    }

    let name_like = find_node_at_offset::<ast::NameLike>(&file, offset)?;
    let name_range = name_like.syntax().text_range();

    let hover_docs_owner = match name_like {
        ast::NameLike::NameRef(name_ref) => {
            let ref_element = name_ref.syntax().ancestor_strict::<ast::ReferenceElement>()?;
            if let Some(result_hover) =
                docs_for_item_spec_fun_result(&sema, ref_element.clone(), file_id)
            {
                return Some(result_hover);
            }
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
    named_element.header(&sema, &mut doc_string);
    writeln!(doc_string).ok()?;
    named_element.signature(&sema, &mut doc_string);
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

fn docs_for_item_spec_fun_result(
    sema: &Semantics<'_, RootDatabase>,
    reference: ast::ReferenceElement,
    file_id: FileId,
) -> Option<RangeInfo<HoverResult>> {
    if reference.reference_name().is_none_or(|it| it != "result") {
        return None;
    }
    let path = reference.path()?;
    if let Some(item_spec) = path.syntax().ancestor_strict::<ast::ItemSpec>() {
        if let Some(fun) = item_spec
            .in_file(file_id)
            .item(sema.db)
            .and_then(|it| it.cast_into::<ast::Fun>())
        {
            // fetch the return type
            let fun_ty = ty_db::lower_function(sema.db, fun, true);
            let fun_ret_ty = sema.render_ty_for_ui(&fun_ty.ret_type_ty(), file_id);
            return Some(RangeInfo::new(
                path.syntax().text_range(),
                HoverResult {
                    doc_string: stdx::trim_indent(&format!(
                        r#"
                        ```
                        result: {fun_ret_ty}
                        ```
                        ---
                        `result` is a special spec variable which holds the return value of a function.
                        "#,
                    )),
                },
            ));
        }
    }
    None
}
