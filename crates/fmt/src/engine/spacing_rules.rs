use syntax::SyntaxKind;
use syntax::parse::token_set::TokenSet;

#[derive(Debug)]
struct RuleCondition {
    parent: Option<TokenSet>,
    left: Option<TokenSet>,
    right: Option<TokenSet>,
}

impl RuleCondition {
    fn matches(&self, parent: SyntaxKind, left: Option<SyntaxKind>, right: SyntaxKind) -> bool {
        self.parent.map_or(true, |p| p.contains(parent))
            && self
                .left
                .map_or(true, |l| left.map_or(false, |lk| l.contains(lk)))
            && self.right.map_or(true, |r| r.contains(right))
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Spacing {
    Spaces(usize),
    LineBreak,
    /// Use spaces unless the original whitespace already contains a line break.
    SpacesOrPreserveLineBreak(usize),
    /// Use `spaces` if the parent fits on one line, otherwise insert a newline.
    SpacesOrLineBreak(usize),
    /// Use `spaces` if the parent contains no newlines, otherwise insert a newline.
    /// "Chop down" style: once any sibling in the parent breaks, this breaks too.
    DependentLineBreak(usize),
}

#[derive(Debug)]
pub(crate) struct SpacingRule {
    conditions: Vec<RuleCondition>,
    pub(crate) spacing: Spacing,
}

impl SpacingRule {
    pub(crate) fn matches(
        &self,
        parent: SyntaxKind,
        left: Option<SyntaxKind>,
        right: SyntaxKind,
    ) -> bool {
        self.conditions.iter().any(|c| c.matches(parent, left, right))
    }
}

pub(crate) struct SpacingRules {
    rules: Vec<SpacingRule>,
}

impl SpacingRules {
    pub(crate) fn new(rules: Vec<SpacingRule>) -> Self {
        SpacingRules { rules }
    }

    pub(crate) fn find_matching_rule(
        &self,
        parent: SyntaxKind,
        left: Option<SyntaxKind>,
        right: SyntaxKind,
    ) -> Option<&SpacingRule> {
        self.rules.iter().find(|rule| rule.matches(parent, left, right))
    }
}

pub(crate) struct RuleBuilder {
    conditions: Vec<RuleCondition>,
}

impl RuleBuilder {
    pub(crate) fn inside(mut self, parent: SyntaxKind) -> RuleBuilder {
        let ts = TokenSet::new(&[parent]);
        for condition in &mut self.conditions {
            condition.parent = Some(ts);
        }
        self
    }

    pub(crate) fn inside_ts(mut self, parents: TokenSet) -> RuleBuilder {
        for condition in &mut self.conditions {
            condition.parent = Some(parents);
        }
        self
    }

    pub(crate) fn spaces(self, count: usize) -> SpacingRule {
        SpacingRule {
            conditions: self.conditions,
            spacing: Spacing::Spaces(count),
        }
    }

    pub(crate) fn line_break(self) -> SpacingRule {
        SpacingRule {
            conditions: self.conditions,
            spacing: Spacing::LineBreak,
        }
    }

    pub(crate) fn spaces_or_preserve_line_break(self, spaces: usize) -> SpacingRule {
        SpacingRule {
            conditions: self.conditions,
            spacing: Spacing::SpacesOrPreserveLineBreak(spaces),
        }
    }

    pub(crate) fn spaces_or_line_break(self, spaces: usize) -> SpacingRule {
        SpacingRule {
            conditions: self.conditions,
            spacing: Spacing::SpacesOrLineBreak(spaces),
        }
    }

    pub(crate) fn dependent_line_break(self, spaces: usize) -> SpacingRule {
        SpacingRule {
            conditions: self.conditions,
            spacing: Spacing::DependentLineBreak(spaces),
        }
    }
}

pub(crate) fn after(kind: SyntaxKind) -> RuleBuilder {
    RuleBuilder {
        conditions: vec![RuleCondition {
            parent: None,
            left: Some(TokenSet::new(&[kind])),
            right: None,
        }],
    }
}

pub(crate) fn after_ts(kinds: TokenSet) -> RuleBuilder {
    RuleBuilder {
        conditions: vec![RuleCondition {
            parent: None,
            left: Some(kinds),
            right: None,
        }],
    }
}

pub(crate) fn before(kind: SyntaxKind) -> RuleBuilder {
    RuleBuilder {
        conditions: vec![RuleCondition {
            parent: None,
            left: None,
            right: Some(TokenSet::new(&[kind])),
        }],
    }
}

pub(crate) fn before_ts(kinds: TokenSet) -> RuleBuilder {
    RuleBuilder {
        conditions: vec![RuleCondition {
            parent: None,
            left: None,
            right: Some(kinds),
        }],
    }
}

pub(crate) fn around(kind: SyntaxKind) -> RuleBuilder {
    let ts = TokenSet::new(&[kind]);
    RuleBuilder {
        conditions: vec![
            RuleCondition {
                parent: None,
                left: None,
                right: Some(ts),
            },
            RuleCondition {
                parent: None,
                left: Some(ts),
                right: None,
            },
        ],
    }
}

pub(crate) fn around_ts(kinds: TokenSet) -> RuleBuilder {
    RuleBuilder {
        conditions: vec![
            RuleCondition {
                parent: None,
                left: None,
                right: Some(kinds),
            },
            RuleCondition {
                parent: None,
                left: Some(kinds),
                right: None,
            },
        ],
    }
}

pub(crate) fn between(left: SyntaxKind, right: SyntaxKind) -> RuleBuilder {
    RuleBuilder {
        conditions: vec![RuleCondition {
            parent: None,
            left: Some(TokenSet::new(&[left])),
            right: Some(TokenSet::new(&[right])),
        }],
    }
}

pub(crate) fn between_ts(left_ts: TokenSet, right_ts: TokenSet) -> RuleBuilder {
    RuleBuilder {
        conditions: vec![RuleCondition {
            parent: None,
            left: Some(left_ts),
            right: Some(right_ts),
        }],
    }
}
