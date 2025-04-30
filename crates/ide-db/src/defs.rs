use crate::{RootDatabase, SymbolKind, ast_kind_to_symbol_kind};
use lang::Semantics;
use std::collections::HashSet;
use std::sync::LazyLock;
use lang::nameres::scope::VecExt;
use syntax::{AstNode, ast};

static INTEGER_TYPE_IDENTS: LazyLock<HashSet<&str>> =
    LazyLock::new(|| HashSet::from(["u8", "u16", "u32", "u64", "u128", "u256"]));

static BUILTIN_TYPE_IDENTS: LazyLock<HashSet<&str>> = LazyLock::new(|| {
    let mut set = HashSet::from(["vector", "address", "signer", "bool"]);
    set.extend(INTEGER_TYPE_IDENTS.iter());
    set
});

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Definition {
    NamedItem(SymbolKind),
    BuiltinType,
}

pub enum NameClass {
    Definition(Definition),
    // /// `field` in `if let Foo { field } = foo`. Here, `ast::Name` both introduces
    // /// a definition into a local scope, and refers to an existing definition.
    // PatFieldShorthand {
    //     local_def: Local,
    //     field_ref: Field,
    //     adt_subst: GenericSubstitution,
    // },
}

impl NameClass {
    pub fn classify(name: &ast::Name) -> Option<NameClass> {
        let parent = name.syntax().parent()?;
        let symbol_kind = ast_kind_to_symbol_kind(parent.kind())?;
        Some(NameClass::Definition(Definition::NamedItem(symbol_kind)))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NameRefClass {
    Definition(Definition),
    // FieldShorthand {
    //     local_ref: ast::IdentPat,
    //     field_ref: ast::StructField,
    // },
}

impl NameRefClass {
    pub fn classify(
        sema: &Semantics<'_, RootDatabase>,
        name_ref: &ast::NameRef,
    ) -> Option<NameRefClass> {
        let ref_parent = name_ref.syntax().parent()?;

        if let Some(path) = ast::PathSegment::cast(ref_parent.clone()).map(|it| it.parent_path()) {
            let res = sema.resolve(path.into()).single_or_none();
            return match res {
                Some(entry) => {
                    let symbol_kind = ast_kind_to_symbol_kind(entry.node_loc.kind())?;
                    Some(NameRefClass::Definition(Definition::NamedItem(symbol_kind)))
                }
                None => {
                    let ref_name = name_ref.as_string();
                    if BUILTIN_TYPE_IDENTS.contains(ref_name.as_str()) {
                        return Some(NameRefClass::Definition(Definition::BuiltinType));
                    }
                    None
                }
            };
        }

        None
    }
}
