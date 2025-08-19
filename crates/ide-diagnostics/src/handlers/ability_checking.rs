use crate::DiagnosticsContext;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use ide_db::Severity;
use lang::types::abilities::Ability;
use lang::types::fold::TypeFoldable;
use syntax::ast;
use syntax::files::InFile;

pub(crate) fn struct_field_type_ability_check(
    acc: &mut Vec<Diagnostic>,
    ctx: &DiagnosticsContext<'_>,
    named_field: InFile<ast::NamedField>,
) -> Option<()> {
    let _p = tracing::debug_span!("struct_field_type_ability_check").entered();

    let field_type = named_field.as_ref().and_then(|it| it.type_())?;
    let field_ty = ctx.sema.lower_type(field_type, false);

    // if type param is involved, should check at call site
    if field_ty.has_ty_type_param() || field_ty.has_ty_infer() {
        return None;
    }
    let field_ty_abilities = field_ty.abilities(ctx.sema.db)?;

    let struct_abilities = named_field
        .value
        .fields_owner()
        .struct_()?
        .abilities()
        .iter()
        .filter_map(|it| Ability::from_ast(it))
        .collect::<Vec<_>>();

    let missing_ability_pairs = vec![
        (Ability::Key, Ability::Store),
        (Ability::Copy, Ability::Copy),
        (Ability::Drop, Ability::Drop),
    ];
    for (struct_ability, req_field_ability) in missing_ability_pairs.iter() {
        if struct_abilities.contains(&struct_ability) && !field_ty_abilities.contains(&req_field_ability)
        {
            acc.push(Diagnostic::new(
                DiagnosticCode::Lsp("missing-ability", Severity::Error),
                format!("Missing required ability `{}`", req_field_ability),
                named_field.file_range(),
            ));
            return Some(());
        }
    }

    Some(())
}
