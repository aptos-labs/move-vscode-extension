use crate::parse::parser::Parser;
use crate::parse::token_set::TokenSet;
use crate::{SyntaxKind, T, ts};
use std::collections::HashSet;
use std::ops::{Add, BitOr};

#[derive(Debug, Clone)]
pub enum RecoveryToken {
    SyntaxKind(SyntaxKind),
    KwIdent(String),
}

impl From<SyntaxKind> for RecoveryToken {
    fn from(value: SyntaxKind) -> Self {
        RecoveryToken::SyntaxKind(value)
    }
}

impl From<&str> for RecoveryToken {
    fn from(value: &str) -> Self {
        RecoveryToken::KwIdent(value.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct RecoverySet {
    pub token_set: TokenSet,
    keywords: HashSet<String>,
}

impl From<SyntaxKind> for RecoverySet {
    fn from(value: SyntaxKind) -> Self {
        RecoverySet::from_ts(value.into())
    }
}

impl From<TokenSet> for RecoverySet {
    fn from(value: TokenSet) -> Self {
        RecoverySet::from_ts(value)
    }
}

impl RecoverySet {
    pub(crate) fn new() -> Self {
        RecoverySet {
            token_set: TokenSet::EMPTY,
            keywords: HashSet::new(),
        }
    }

    pub(crate) fn from_ts(token_set: TokenSet) -> Self {
        let mut rset = Self::new();
        rset.token_set = token_set;
        rset
    }

    pub(crate) fn with_merged(mut self, other: RecoverySet) -> Self {
        self.token_set = self.token_set.union(other.token_set);
        self.keywords.extend(other.keywords);
        self
    }

    pub(crate) fn with_token_set(mut self, token_set: impl Into<TokenSet>) -> Self {
        self.token_set = self.token_set + token_set.into();
        self
    }

    pub(crate) fn with_kw(mut self, kw_ident: &str) -> Self {
        self.keywords.insert(kw_ident.to_string());
        self
    }

    pub(crate) fn with_recovery_token(mut self, recovery_token: RecoveryToken) -> Self {
        match recovery_token {
            RecoveryToken::SyntaxKind(t) => {
                self.token_set = self.token_set | t;
            }
            RecoveryToken::KwIdent(kw) => {
                self.keywords.insert(kw);
            }
        }
        self
    }

    pub(crate) fn without_recovery_token(mut self, recovery_token: RecoveryToken) -> Self {
        match recovery_token {
            RecoveryToken::SyntaxKind(t) => {
                self.token_set = self.token_set.sub(ts!(t));
            }
            RecoveryToken::KwIdent(kw) => {
                self.keywords.remove(&kw);
            }
        };
        self
    }

    pub(crate) fn contains(&self, t: SyntaxKind) -> bool {
        self.token_set.contains(t)
    }

    pub(crate) fn contains_current(&self, p: &Parser) -> bool {
        match p.current() {
            T![ident] => {
                if self.token_set.contains(T![ident]) {
                    return true;
                }
                let current_text = p.current_text();
                self.keywords.contains(current_text)
            }
            kind => self.token_set.contains(kind),
        }
    }
}
