use crate::ast;

impl ast::TypeParam {
    pub fn ability_bounds(&self) -> Vec<ast::Ability> {
        self.ability_bound_list()
            .map(|it| it.abilities().collect())
            .unwrap_or_default()
    }
}
