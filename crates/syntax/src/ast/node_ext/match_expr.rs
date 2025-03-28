use crate::ast;

impl ast::MatchExpr {
    pub fn arms(&self) -> Vec<ast::MatchArm> {
        self.match_arm_list()
            .map(|it| it.match_arms().collect())
            .unwrap_or_default()
    }
}
