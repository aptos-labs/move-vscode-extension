use crate::ast;
use crate::ast::AstChildren;

impl ast::AbilityContainer {
    pub fn abilities(&self) -> AstChildren<ast::Ability> {
        match self {
            ast::AbilityContainer::AbilityBoundList(ability_bound_list) => {
                ability_bound_list.abilities()
            }
            ast::AbilityContainer::AbilityList(ability_list) => ability_list.abilities(),
        }
    }
}
