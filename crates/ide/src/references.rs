// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::NavigationTarget;
use ide_db::defs::{Definition, NameClass, NameRefClass};
use ide_db::search::SearchScope;
use ide_db::{RootDatabase, search};
use itertools::Itertools;
use lang::Semantics;
use std::collections::HashMap;
use syntax::SyntaxKind::*;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::{FilePosition, InFile};
use syntax::{AstNode, SyntaxNode, TextRange, TextSize, ast};
use vfs::FileId;

/// Result of a reference search operation.
#[derive(Debug, Clone)]
pub struct ReferenceSearchResult {
    /// Information about the declaration site of the searched item.
    /// For ADTs (structs/enums), this points to the type definition.
    /// May be None for primitives or items without clear declaration sites.
    pub declaration: Option<NavigationTarget>,
    /// All references found, grouped by file.
    /// For ADTs when searching from a constructor position (e.g. on '{', '(', ';'),
    /// this only includes constructor/initialization usages.
    /// The map key is the file ID, and the value is a vector of `range`.
    /// - range: The text range of the reference in the file
    pub references: HashMap<FileId, Vec<TextRange>>,
}

/// Information about the declaration site of a searched item.
#[derive(Debug, Clone)]
pub struct Declaration {
    /// Navigation information to jump to the declaration
    pub nav: NavigationTarget,
}

pub(crate) fn find_all_refs<'a>(
    db: &RootDatabase,
    position: FilePosition,
    search_scope: Option<SearchScope>,
) -> Option<ReferenceSearchResult> {
    let _p = tracing::info_span!("find_all_refs").entered();

    let sema = Semantics::new(db, position.file_id);

    let tree = sema.parse(position.file_id).syntax().clone();
    let named_item = find_def_at_offset(&sema, &tree, position.offset)?;

    let usages = search::item_usages(&sema, named_item.clone())
        .set_scope(search_scope)
        .fetch_all();
    let references: HashMap<FileId, Vec<TextRange>> = usages
        .into_iter()
        .map(|(file_id, refs)| {
            (
                file_id,
                refs.into_iter().map(|file_ref| file_ref.range).unique().collect(),
            )
        })
        .collect();

    let declaration = NavigationTarget::from_named_item(named_item);
    Some(ReferenceSearchResult { declaration, references })
}

pub(crate) fn find_def_at_offset(
    sema: &Semantics<'_, RootDatabase>,
    tree: &SyntaxNode,
    offset: TextSize,
) -> Option<InFile<ast::NamedElement>> {
    let token = tree
        .token_at_offset(offset)
        .find(|t| matches!(t.kind(), IDENT | INT_NUMBER | QUOTE_IDENT))?;

    let name_like = token.parent()?.cast::<ast::NameLike>()?;
    match name_like {
        ast::NameLike::NameRef(name_ref) => {
            match NameRefClass::classify(sema, &name_ref)? {
                NameRefClass::Definition(Definition::NamedItem(_, named_item)) => Some(named_item),
                // NameRefClass::FieldShorthand {
                //     ident_pat,
                //     named_field,
                // } => Some(ident_pat.named_element()),
                _ => None,
            }
        }
        ast::NameLike::Name(name) => match NameClass::classify(sema, name)? {
            NameClass::Definition(Definition::NamedItem(_, named_item)) => Some(named_item),
            NameClass::PatFieldShorthand { ident_pat, .. } => Some(ident_pat.map_into()),
            _ => None,
        },
    }
}
