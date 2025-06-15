use ide_db::RootDatabase;
use lang::Semantics;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::files::{FilePosition, InFileExt};
use syntax::{AstNode, algo, ast};

pub(crate) fn expr_type_info(
    db: &RootDatabase,
    FilePosition { file_id, offset }: FilePosition,
) -> Option<String> {
    let sema = Semantics::new(db, file_id);

    let file = sema.parse(file_id);

    let expr = algo::find_node_at_offset::<ast::Expr>(file.syntax(), offset)?;
    let expr_ty = sema.get_expr_type(&expr.in_file(file_id))?;

    Some(expr_ty.render(db, None))
}
