use crate::{RootDatabase, SymbolKind, ast_kind_to_symbol_kind};
use lang::Semantics;
use lang::nameres::scope::VecExt;
use std::collections::HashSet;
use std::sync::LazyLock;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::InFile;
use syntax::{AstNode, SyntaxNode, SyntaxToken, ast, match_ast};

static INTEGER_TYPE_IDENTS: LazyLock<HashSet<&str>> =
    LazyLock::new(|| HashSet::from(["u8", "u16", "u32", "u64", "u128", "u256"]));

static BUILTIN_TYPE_IDENTS: LazyLock<HashSet<&str>> = LazyLock::new(|| {
    let mut set = HashSet::from(["vector", "address", "signer", "bool"]);
    set.extend(INTEGER_TYPE_IDENTS.iter());
    set
});

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Definition {
    NamedItem(SymbolKind, InFile<ast::AnyNamedElement>),
    BuiltinType,
}

#[derive(Debug)]
pub enum IdentClass {
    NameClass(NameClass),
    NameRefClass(NameRefClass),
}

impl IdentClass {
    pub fn classify_node(sema: &Semantics<'_, RootDatabase>, node: &SyntaxNode) -> Option<IdentClass> {
        match_ast! {
            match node {
                ast::Name(name) => NameClass::classify(sema, name).map(IdentClass::NameClass),
                ast::NameRef(name_ref) => NameRefClass::classify(sema, &name_ref).map(IdentClass::NameRefClass),
                _ => None,
            }
        }
    }

    pub fn classify_token(
        sema: &Semantics<'_, RootDatabase>,
        token: &SyntaxToken,
    ) -> Option<IdentClass> {
        let parent = token.parent()?;
        Self::classify_node(sema, &parent)
    }
}

#[derive(Debug)]
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
    pub fn classify(sema: &Semantics<'_, RootDatabase>, name: ast::Name) -> Option<NameClass> {
        let name = sema.wrap_node_infile(name);
        let named_item = name.and_then(|it| it.syntax().parent_of_type::<ast::AnyNamedElement>())?;
        let symbol_kind = ast_kind_to_symbol_kind(named_item.kind())?;
        Some(NameClass::Definition(Definition::NamedItem(
            symbol_kind,
            named_item,
        )))
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
                    let named_item = entry.node_loc.to_ast::<ast::AnyNamedElement>(sema.db)?;
                    let symbol_kind = ast_kind_to_symbol_kind(named_item.kind())?;
                    Some(NameRefClass::Definition(Definition::NamedItem(
                        symbol_kind,
                        named_item,
                    )))
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
