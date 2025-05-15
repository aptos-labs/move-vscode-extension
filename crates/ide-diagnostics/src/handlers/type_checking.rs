use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use ide_db::Severity;
use lang::hir_db::NodeInferenceExt;
use lang::types::inference::TypeError;
use lang::types::ty::Ty;
use syntax::ast;
use syntax::files::{FileRange, InFile};
use vfs::FileId;

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn type_check(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    inference_ctx_owner: &InFile<ast::InferenceCtxOwner>,
) -> Option<()> {
    let inference = ctx.sema.inference(inference_ctx_owner, false)?;
    let file_id = inference_ctx_owner.file_id;
    for type_error in &inference.type_errors {
        register_type_error(acc, ctx, file_id, type_error);
    }
    Some(())
}

fn register_type_error(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    file_id: FileId,
    type_error: &TypeError,
) {
    match type_error {
        TypeError::TypeMismatch { loc, actual_ty, expected_ty } => {
            let actual = ctx.sema.render_ty(actual_ty);
            let expected = ctx.sema.render_ty(expected_ty);
            acc.push(Diagnostic::new(
                DiagnosticCode::Lsp("type-error", Severity::Error),
                format!("Incompatible type '{actual}', expected '{expected}'"),
                FileRange {
                    file_id,
                    range: loc.text_range(),
                },
            ))
        }
        TypeError::UnsupportedArithmOp { loc, ty, op } => {
            let ty = ctx.sema.render_ty(ty);
            acc.push(Diagnostic::new(
                DiagnosticCode::Lsp("type-error", Severity::Error),
                format!("Invalid argument to {op}: expected integer type, but found {ty}"),
                FileRange {
                    file_id,
                    range: loc.text_range(),
                },
            ))
        }
        TypeError::InvalidUnpacking { loc, assigned_ty } => {
            use syntax::SyntaxKind::*;

            let message = match loc.kind() {
                STRUCT_PAT if !matches!(assigned_ty, Ty::Adt(_) | Ty::Tuple(_)) => {
                    format!(
                        "Assigned expr of type '{}' cannot be unpacked with struct pattern",
                        ctx.sema.render_ty(assigned_ty)
                    )
                }
                TUPLE_PAT if !matches!(assigned_ty, Ty::Adt(_) | Ty::Tuple(_)) => {
                    format!(
                        "Assigned expr of type '{}' cannot be unpacked with tuple pattern",
                        ctx.sema.render_ty(assigned_ty)
                    )
                }
                _ => {
                    format!(
                        "Invalid unpacking. Expected {}",
                        ctx.sema.render_ty_expected_form(assigned_ty)
                    )
                }
            };
            acc.push(Diagnostic::new(
                DiagnosticCode::Lsp("type-error", Severity::Error),
                message,
                FileRange {
                    file_id,
                    range: loc.text_range(),
                },
            ))
        }
    }
}
