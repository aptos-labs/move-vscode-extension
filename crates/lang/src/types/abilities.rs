use crate::types::ty::Ty;
use crate::types::ty::range_like::TySequence;
use crate::types::ty::ty_var::TyInfer;
use base_db::SourceDatabase;
use std::fmt;
use std::fmt::Formatter;
use syntax::ast;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Ability {
    Key,
    Store,
    Copy,
    Drop,
}

impl fmt::Display for Ability {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Ability::Copy => f.write_str("copy"),
            Ability::Store => f.write_str("store"),
            Ability::Key => f.write_str("key"),
            Ability::Drop => f.write_str("drop"),
        }
    }
}

impl Ability {
    pub fn from_ast(ability: &ast::Ability) -> Option<Ability> {
        let ability_name = ability.ident_token().to_string();
        let ability = match ability_name.as_str() {
            "copy" => Ability::Copy,
            "store" => Ability::Store,
            "drop" => Ability::Drop,
            "key" => Ability::Key,
            _ => {
                return None;
            }
        };
        Some(ability)
    }

    pub fn all() -> Vec<Ability> {
        vec![Ability::Store, Ability::Key, Ability::Drop, Ability::Copy]
    }
}

impl Ty {
    pub fn abilities(&self, db: &dyn SourceDatabase) -> Option<Vec<Ability>> {
        match self {
            // primitives
            Ty::Bool | Ty::Integer(_) | Ty::Address | Ty::Num | Ty::Bv => {
                Some(vec![Ability::Copy, Ability::Drop, Ability::Store])
            }
            Ty::Infer(TyInfer::IntVar(_)) => Some(vec![Ability::Copy, Ability::Drop, Ability::Store]),
            Ty::Seq(TySequence::Range(_)) => Some(vec![Ability::Copy, Ability::Drop, Ability::Store]),
            Ty::Signer => Some(vec![Ability::Drop]),
            Ty::Unit => Some(vec![Ability::Copy, Ability::Copy]),
            Ty::Never => Some(vec![]),
            Ty::Unknown => Some(Ability::all()),
            Ty::Adt(ty_adt) => {
                let adt_item = ty_adt.adt_item(db)?;
                let abilities = adt_item
                    .value
                    .abilities()
                    .iter()
                    .filter_map(|it| Ability::from_ast(it))
                    .collect::<Vec<_>>();
                Some(abilities)
            }
            Ty::TypeParam(ty_type_param) => {
                let type_param = ty_type_param.origin_type_param(db)?;
                let abilities = type_param
                    .value
                    .ability_bounds()
                    .into_iter()
                    .filter_map(|it| Ability::from_ast(&it))
                    .collect::<Vec<_>>();
                Some(abilities)
            }
            Ty::Seq(TySequence::Vector(item_ty)) => item_ty.abilities(db),
            Ty::Infer(TyInfer::Var(_)) => Some(Ability::all()),
            Ty::Reference(_) => Some(vec![Ability::Drop, Ability::Copy]),
            // todo:
            _ => Some(Ability::all()),
        }
    }
}
