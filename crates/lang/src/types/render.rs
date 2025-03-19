use stdx::itertools::Itertools;
use crate::types::ty::adt::TyAdt;
use crate::types::ty::type_param::TyTypeParameter;
use crate::types::ty::Ty;
use crate::AsName;
use syntax::ast::HasName;

pub fn render(ty: &Ty) -> String {
    match ty {
        Ty::Vector(ty) => {
            format!("vector<{}>", render(ty))
        }
        Ty::Adt(ty_adt) => render_ty_adt(ty_adt),
        Ty::Reference(ty_ref) => {
            let prefix = if ty_ref.is_mut { "&mut " } else { "&" };
            let inner = render(ty_ref.referenced());
            format!("{}{}", prefix, inner)
        }
        Ty::Tuple(ty_tuple) => {
            let rendered_tys = ty_tuple.types.iter().map(|it| render(it)).join(", ");
            format!("({})", rendered_tys)
        }

        Ty::TypeParam(ty_tp) => render_ty_tp(ty_tp),
        Ty::Var(ty_var) => ty_var.to_string(),

        Ty::Bool => "bool".to_string(),
        Ty::Signer => "signer".to_string(),
        Ty::Address => "address".to_string(),
        Ty::Integer(kind) => kind.to_string(),
        Ty::Num => "num".to_string(),

        Ty::Unit => "()".to_string(),
        Ty::Unknown => unknown(),
        Ty::Never => never(),
    }
}

fn render_ty_adt(_ty_adt: &TyAdt) -> String {
    "_".to_string()
}

fn render_ty_tp(type_param: &TyTypeParameter) -> String {
    let name = type_param.origin.name().map(|it| it.as_name().to_string());
    name.unwrap_or(anonymous())
}

fn unknown() -> String {
    "<unknown>".to_string()
}

fn never() -> String {
    "<never>".to_string()
}

fn anonymous() -> String {
    "<anonymous>".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ty::IntegerKind;
    use crate::types::ty::ty_var::TyVar;

    #[test]
    fn render_vector() {
        let ty = Ty::Vector(Box::new(Ty::Bool));
        let rendered = render(&ty);
        assert_eq!(rendered, "vector<bool>");
    }

    #[test]
    fn render_ty_var() {
        assert_eq!(render(&Ty::Var(TyVar::new_anonymous(0))), "?_0");
    }

    #[test]
    fn render_ty_integer() {
        assert_eq!(render(&Ty::Integer(IntegerKind::Integer)), "integer");
        assert_eq!(render(&Ty::Integer(IntegerKind::U8)), "u8");
        assert_eq!(render(&Ty::Integer(IntegerKind::U64)), "u64");
    }
}
