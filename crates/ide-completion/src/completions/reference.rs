mod method_or_field;
mod paths;

use crate::completions::Completions;
use crate::completions::reference::method_or_field::add_method_or_field_completions;
use crate::completions::reference::paths::add_path_completions;
use crate::context::{CompletionContext, ReferenceKind};
use lang::InFile;
use std::cell::RefCell;
use syntax::{AstNode, ast};

pub(crate) fn add_reference_completions(
    completions: &RefCell<Completions>,
    ctx: &CompletionContext<'_>,
    reference_kind: ReferenceKind,
) -> Option<()> {
    use syntax::SyntaxKind::*;

    match reference_kind {
        ReferenceKind::Path(path) => add_path_completions(completions, ctx, path),
        ReferenceKind::FieldRef { receiver_expr } => {
            add_method_or_field_completions(completions, ctx, receiver_expr)
        }
    }
}
