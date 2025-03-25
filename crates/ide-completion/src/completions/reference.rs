mod method_or_field;
mod paths;

use crate::completions::reference::method_or_field::add_method_or_field_completions;
use crate::completions::reference::paths::add_path_completions;
use crate::completions::Completions;
use crate::context::CompletionContext;
use lang::InFile;
use std::cell::RefCell;
use syntax::{ast, AstNode};

pub(crate) fn add_reference_completions(
    completions: &RefCell<Completions>,
    ctx: &CompletionContext<'_>,
    ref_element: InFile<ast::AnyReferenceElement>,
) -> Option<()> {
    use syntax::SyntaxKind::*;

    match ref_element.kind() {
        PATH => add_path_completions(completions, ctx, ref_element.cast::<ast::Path>().unwrap()),
        FIELD_REF => add_method_or_field_completions(
            completions,
            ctx,
            ref_element.cast::<ast::FieldRef>().unwrap(),
        ),
        _ => None,
    }
}
