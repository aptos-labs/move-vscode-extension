use std::fmt;
use syntax::ast;

// todo: add symbol interning some time in the future, see rust-analyzer impl
type Symbol = String;

/// `Name` is a wrapper around string, which is used in lang for both references
/// and declarations.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Name {
    symbol: Symbol,
}

impl fmt::Debug for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Name")
            .field("symbol", &self.symbol.as_str())
            .finish()
    }
}

impl Ord for Name {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.symbol.as_str().cmp(other.symbol.as_str())
    }
}

impl PartialOrd for Name {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq<Name> for Symbol {
    fn eq(&self, name: &Name) -> bool {
        *self == name.symbol
    }
}

impl Name {
    pub fn new(text: &str) -> Name {
        Name {
            symbol: text.to_string(),
            // ast,
        }
    }

    /// Returns the text this name represents if it isn't a tuple field.
    pub fn as_str(&self) -> &str {
        self.symbol.as_str()
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str().to_string())
    }
}

pub trait AsName {
    fn as_name(&self) -> Name;
}

impl AsName for ast::NameRef {
    fn as_name(&self) -> Name {
        Name::new(&self.text())
    }
}

impl AsName for ast::Name {
    fn as_name(&self) -> Name {
        Name::new(&self.text())
        // Name::new(&self.text(), ast::NameLike::Name(self.to_owned()))
    }
}
