use std::fmt;
use syntax::ast;

// todo: add symbol interning some time in the future, see rust-analyzer impl
type Symbol = String;

/// `Name` is a wrapper around string, which is used in lang for both references
/// and declarations.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Name {
    symbol: Symbol,
    // todo: remove this field eventually
    // ast: ast::NameLike,
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

    // pub fn new_tuple_field(idx: usize) -> Name {
    //     Name {
    //         symbol: idx.to_string(),
    //     }
    // }

    // /// A fake name for things missing in the source code.
    // ///
    // /// For example, `impl Foo for {}` should be treated as a trait impl for a
    // /// type with a missing name. Similarly, `struct S { : u32 }` should have a
    // /// single field with a missing name.
    // ///
    // /// Ideally, we want a `gensym` semantics for missing names -- each missing
    // /// name is equal only to itself. It's not clear how to implement this in
    // /// salsa though, so we punt on that bit for a moment.
    // pub fn missing() -> Name {
    //     Name {
    //         symbol: MISSING_NAME.to_string(),
    //     }
    // }

    // /// Returns true if this is a fake name for things missing in the source code. See
    // /// [`missing()`][Self::missing] for details.
    // ///
    // /// Use this method instead of comparing with `Self::missing()` as missing names
    // /// (ideally should) have a `gensym` semantics.
    // pub fn is_missing(&self) -> bool {
    //     self == &Name::missing()
    // }

    // /// Returns the tuple index this name represents if it is a tuple field.
    // pub fn as_tuple_index(&self) -> Option<usize> {
    //     self.symbol.as_str().parse().ok()
    // }

    /// Returns the text this name represents if it isn't a tuple field.
    pub fn as_str(&self) -> &str {
        self.symbol.as_str()
    }

    // pub fn syntax(&self) -> &SyntaxNode {
    //     match &self.ast {
    //         ast::NameLike::NameRef(name_ref) => name_ref.syntax(),
    //         ast::NameLike::Name(name) => name.syntax(),
    //     }
    // }

    // pub fn symbol(&self) -> &Symbol {
    //     &self.symbol
    // }

    // #[inline]
    // pub fn eq_ident(&self, ident: &str) -> bool {
    //     self.as_str() == ident
    // }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str().to_string())
    }
}

pub trait AsName {
    fn as_name(&self) -> Name;
}

impl AsName for ast::NameRef {
    fn as_name(&self) -> Name {
        Name::new(&self.text())
        // Name::new(&self.text(), ast::NameLike::NameRef(self.to_owned()))
        // match self.as_tuple_field() {
        //     Some(idx) => Name::new_tuple_field(idx),
        //     None => Name::new(&self.text(), self.to_owned()),
        // }
    }
}

impl AsName for ast::Name {
    fn as_name(&self) -> Name {
        Name::new(&self.text())
        // Name::new(&self.text(), ast::NameLike::Name(self.to_owned()))
    }
}
