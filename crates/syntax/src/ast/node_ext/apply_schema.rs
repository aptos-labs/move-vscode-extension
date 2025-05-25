use crate::ast;
use crate::ast::WildcardPattern;

impl ast::ApplySchema {
    pub fn apply_to_patterns(&self) -> Vec<WildcardPattern> {
        self.apply_to()
            .map(|it| it.wildcards().collect())
            .unwrap_or_default()
    }

    pub fn apply_except_patterns(&self) -> Vec<WildcardPattern> {
        self.apply_except()
            .map(|it| it.wildcards().collect())
            .unwrap_or_default()
    }
}
