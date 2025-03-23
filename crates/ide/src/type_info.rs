use base_db::Upcast;
use ide_db::RootDatabase;
use lang::db::HirDatabase;
use lang::files::InFileExt;
use lang::types::ty::Ty;
use lang::{FilePosition, InFile, Semantics};
use syntax::{algo, ast, AstNode};

pub(crate) fn expr_type_info(
    db: &RootDatabase,
    FilePosition { file_id, offset }: FilePosition,
) -> Option<String> {
    let sema = Semantics::new(db);

    let file = sema.parse(file_id);

    let expr = algo::find_node_at_offset::<ast::Expr>(file.syntax(), offset)?;

    let ctx_owner = expr.clone().inference_ctx_owner();
    let expr_ty = match ctx_owner {
        None => Ty::Unknown,
        Some(ctx_owner) => db
            .inference(ctx_owner.in_file(file_id))
            .get_expr_type(&expr)
            .unwrap_or(Ty::Unknown),
    };
    Some(expr_ty.render(db.upcast()))
}
