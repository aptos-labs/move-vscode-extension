use crate::{RootDatabase, SymbolKind, ast_kind_to_symbol_kind};
use lang::Semantics;
use lang::nameres::scope::VecExt;
use std::collections::HashSet;
use std::sync::LazyLock;
use syntax::SyntaxKind::*;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
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

impl Definition {
    pub fn from_named_item(named_item: InFile<impl Into<ast::AnyNamedElement>>) -> Option<Definition> {
        let named_item: InFile<ast::AnyNamedElement> = named_item.map(|it| it.into());
        let symbol_kind = ast_kind_to_symbol_kind(named_item.kind())?;
        Some(Definition::NamedItem(symbol_kind, named_item))
    }
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
    /// `None` in `if let None = Some(82) {}`.
    /// Syntactically, it is a name, but semantically it is a reference.
    ConstReference(Definition),
    /// `field` in `if let Foo { field } = foo`. Here, `ast::IdentPat` both introduces
    /// a definition into a local scope, and refers to an existing definition.
    PatFieldShorthand {
        ident_pat: InFile<ast::IdentPat>,
        named_field: InFile<ast::NamedField>,
    },
    ItemSpecFunctionParam {
        spec_ident_pat: InFile<ast::IdentPat>,
        fun_param_ident_pat: InFile<ast::IdentPat>,
    },
}

impl NameClass {
    pub fn classify(sema: &Semantics<'_, RootDatabase>, name: ast::Name) -> Option<NameClass> {
        let _p = tracing::info_span!("NameClass::classify").entered();

        let named_item = name.syntax().parent_of_type::<ast::AnyNamedElement>()?;
        match_ast! {
            match (named_item.syntax()) {
                ast::IdentPat(ident_pat) => Self::classify_ident_pat(sema, ident_pat),
                _ => {
                    let name = sema.wrap_node_infile(name);
                    let named_item = name.and_then(|it| it.syntax().parent_of_type::<ast::AnyNamedElement>())?;
                    let defn = Definition::from_named_item(named_item)?;
                    Some(NameClass::Definition(defn))
                },
            }
        }
    }

    fn classify_ident_pat(
        sema: &Semantics<'_, RootDatabase>,
        ident_pat: ast::IdentPat,
    ) -> Option<NameClass> {
        let ident_pat = sema.wrap_node_infile(ident_pat);
        if let Some(resolved_ident_pat) =
            sema.resolve_to_element::<ast::AnyNamedElement>(ident_pat.clone().map_into())
        {
            if matches!(resolved_ident_pat.kind(), CONST | VARIANT) {
                let defn = Definition::from_named_item(resolved_ident_pat)?;
                return Some(NameClass::ConstReference(defn));
            }

            // item spec function param
            if let Some(fun_ident_pat) = resolved_ident_pat.cast_into_ref::<ast::IdentPat>() {
                if fun_ident_pat.value.syntax().parent_is::<ast::Param>() {
                    return Some(NameClass::ItemSpecFunctionParam {
                        spec_ident_pat: ident_pat,
                        fun_param_ident_pat: fun_ident_pat,
                    });
                }
            }

            let pat_parent = ident_pat.value.syntax().parent();
            if let Some(struct_pat_field) = pat_parent.and_then(|it| it.cast::<ast::StructPatField>()) {
                if struct_pat_field.name_ref().is_none() {
                    if let Some(named_field) = resolved_ident_pat.cast_into_ref::<ast::NamedField>() {
                        return Some(NameClass::PatFieldShorthand { ident_pat, named_field });
                    }
                }
            }
        }

        let local_def = Definition::from_named_item(ident_pat)?;
        Some(NameClass::Definition(local_def))
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
                    let defn = Definition::from_named_item(named_item)?;
                    Some(NameRefClass::Definition(defn))
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
