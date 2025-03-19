use crate::types::lowering::TyLowering;
use crate::types::substitution::Substitution;
use crate::types::ty::type_param::TyTypeParameter;
use crate::types::ty::Ty;
use crate::InFile;
use std::collections::HashMap;
use syntax::ast::HasTypeParams;
use syntax::{ast, AstNode};

impl TyLowering<'_> {
    pub fn type_args_substitution(
        &self,
        path: ast::Path,
        generic_item: InFile<ast::AnyHasTypeParams>,
    ) -> Substitution {
        let mut subst_mapping = HashMap::new();

        let psi_subst = psi_type_args_subst(path, generic_item.value.type_params());
        for (type_param, psi_type_arg) in psi_subst {
            let type_param = InFile::new(generic_item.file_id, type_param);
            let type_param_ty = TyTypeParameter::new(type_param);
            let ty = match psi_type_arg {
                PsiTypeArg::Present(type_) => self.lower_type(type_),
                PsiTypeArg::OptionalAbsent => Ty::TypeParam(type_param_ty.clone()),
                PsiTypeArg::RequiredAbsent => Ty::Unknown,
            };
            subst_mapping.insert(type_param_ty, ty);
        }

        Substitution::new(subst_mapping)
    }
}

fn psi_type_args_subst(
    path: ast::Path,
    type_params: Vec<ast::TypeParam>,
) -> HashMap<ast::TypeParam, PsiTypeArg> {
    let root_parent = path.root_path().syntax().parent().unwrap();

    // Generic arguments are optional in expression context, e.g.
    // `let a = Foo::<u8>::bar::<u16>();` can be written as `let a = Foo::bar();`
    // if it is possible to infer `u8` and `u16` during type inference
    let is_args_optional =
        ast::Expr::can_cast(root_parent.kind()) || ast::Pat::can_cast(root_parent.kind());

    let type_args_list = path.segment().type_arg_list();
    if type_args_list.is_none() {
        let type_arg = if is_args_optional {
            PsiTypeArg::OptionalAbsent
        } else {
            PsiTypeArg::RequiredAbsent
        };
        return type_params.into_iter().map(|it| (it, type_arg.clone())).collect();
    }

    let mut type_args = type_args_list
        .unwrap()
        .type_arguments()
        .collect::<Vec<_>>()
        .into_iter();
    let mut subst = HashMap::new();
    for type_param in type_params {
        let type_arg = type_args.next();
        let psi_type_arg = match type_arg {
            Some(type_arg) => PsiTypeArg::Present(type_arg.type_()),
            None => PsiTypeArg::RequiredAbsent,
        };
        subst.insert(type_param, psi_type_arg);
    }
    subst
}

#[derive(Debug, Clone)]
pub enum PsiTypeArg {
    Present(ast::Type),
    RequiredAbsent,
    OptionalAbsent,
}
