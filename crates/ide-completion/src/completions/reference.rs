mod method_or_field;
pub(crate) mod paths;

use crate::completions::Completions;
use crate::completions::reference::method_or_field::add_method_or_field_completions;
use crate::completions::reference::paths::add_path_completions;
use crate::context::{CompletionContext, ReferenceKind};
use std::cell::RefCell;
use syntax::files::InFileExt;

pub(crate) fn add_reference_completions(
    completions: &RefCell<Completions>,
    ctx: &CompletionContext<'_>,
    reference_kind: ReferenceKind,
) -> Option<()> {
    let file_id = ctx.position.file_id;
    match reference_kind {
        ReferenceKind::Path(path) => add_path_completions(completions, ctx, path.in_file(file_id)),
        ReferenceKind::FieldRef { receiver_expr } => {
            add_method_or_field_completions(completions, ctx, receiver_expr.in_file(file_id))
        }
    }
}
