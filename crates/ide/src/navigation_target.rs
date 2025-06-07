use base_db::SourceDatabase;
use ide_db::{RootDatabase, SymbolKind, ast_kind_to_symbol_kind};
use lang::Semantics;
use lang::nameres::scope::ScopeEntry;
use std::fmt;
use syntax::ast::NamedElement;
use syntax::files::InFile;
use syntax::{AstNode, TextRange, ast};
use vfs::FileId;

/// `NavigationTarget` represents an element in the editor's UI which you can
/// click on to navigate to a particular piece of code.
///
/// Typically, a `NavigationTarget` corresponds to some element in the source
/// code, like a function or a struct, but this is not strictly required.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct NavigationTarget {
    pub file_id: FileId,
    /// Range which encompasses the whole element.
    ///
    /// Should include body, doc comments, attributes, etc.
    ///
    /// Clients should use this range to answer "is the cursor inside the
    /// element?" question.
    pub full_range: TextRange,
    /// A "most interesting" range within the `full_range`.
    ///
    /// Typically, `full_range` is the whole syntax node, including doc
    /// comments, and `focus_range` is the range of the identifier.
    ///
    /// Clients should place the cursor on this range when navigating to this target.
    ///
    /// This range must be contained within [`Self::full_range`].
    pub focus_range: Option<TextRange>,
    pub name: String,
    pub kind: Option<SymbolKind>,
    pub container_name: Option<String>,
    pub description: Option<String>,
    // pub docs: Option<Documentation>,
    /// In addition to a `name` field, a `NavigationTarget` may also be aliased
    /// In such cases we want a `NavigationTarget` to be accessible by its alias
    pub alias: Option<String>,
}

impl fmt::Debug for NavigationTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct("NavigationTarget");
        macro_rules! opt {
            ($($name:ident)*) => {$(
                if let Some(it) = &self.$name {
                    f.field(stringify!($name), it);
                }
            )*}
        }
        f.field("file_id", &self.file_id)
            .field("full_range", &self.full_range);
        opt!(focus_range);
        f.field("name", &self.name);
        opt!(kind container_name description);
        f.finish()
    }
}

impl NavigationTarget {
    pub fn focus_or_full_range(&self) -> TextRange {
        self.focus_range.unwrap_or(self.full_range)
    }

    #[cfg(test)]
    pub(crate) fn debug_render(&self) -> String {
        let mut buf = format!(
            "{} {:?} {:?} {:?}",
            self.name,
            self.kind.unwrap(),
            self.file_id,
            self.full_range
        );
        if let Some(focus_range) = self.focus_range {
            buf.push_str(&format!(" {focus_range:?}"))
        }
        if let Some(container_name) = &self.container_name {
            buf.push_str(&format!(" {container_name}"))
        }
        buf
    }

    /// Allows `NavigationTarget` to be created from a `NameOwner`
    pub(crate) fn from_scope_entry<'db>(
        sema: &'db Semantics<'db, RootDatabase>,
        scope_entry: ScopeEntry,
    ) -> Option<NavigationTarget> {
        let entry_name = scope_entry.name;
        let file_id = scope_entry.node_loc.file_id();
        if sema
            .db
            .builtins_file_id()
            .is_some_and(|fid| fid.data(sema.db) == file_id)
        {
            return None;
        }
        if let Some(label_decl) = scope_entry.node_loc.to_ast::<ast::LabelDecl>(sema.db) {
            let label = label_decl.value;
            let name_range = label.quote_ident_token().text_range();
            let node_range = label.syntax().text_range();
            return Some(NavigationTarget::from_syntax(
                file_id,
                entry_name.into(),
                Some(name_range),
                node_range,
                SymbolKind::Label,
            ));
        }
        if let Some(tuple_field) = scope_entry.node_loc.to_ast::<ast::TupleField>(sema.db) {
            let tuple_field = tuple_field.value;
            let node_range = tuple_field.syntax().text_range();
            return Some(NavigationTarget::from_syntax(
                file_id,
                entry_name.into(),
                Some(node_range.clone()),
                node_range,
                SymbolKind::Field,
            ));
        }

        let named_item = scope_entry
            .node_loc
            .to_ast::<ast::AnyNamedElement>(sema.db)?
            .value;
        let name_range = named_item.name().map(|name| name.ident_token().text_range());
        let node_range = named_item.syntax().text_range();

        let kind = ast_kind_to_symbol_kind(named_item.syntax().kind())?;
        Some(NavigationTarget::from_syntax(
            file_id,
            entry_name.into(),
            name_range,
            node_range,
            kind,
        ))
    }

    /// Allows `NavigationTarget` to be created from a `NameOwner`
    pub(crate) fn from_named_item<'db>(
        _sema: &'db Semantics<'db, RootDatabase>,
        named_item: InFile<ast::AnyNamedElement>,
    ) -> Option<NavigationTarget> {
        let (file_id, named_item) = named_item.unpack();

        let name = named_item.name()?;

        let name_s = name.as_string();
        let name_range = name.ident_token().text_range();
        let node_range = named_item.syntax().text_range();

        let kind = ast_kind_to_symbol_kind(named_item.syntax().kind())?;
        Some(NavigationTarget::from_syntax(
            file_id,
            name_s,
            Some(name_range),
            node_range,
            kind,
        ))
    }

    fn from_syntax(
        file_id: FileId,
        name: String,
        focus_range: Option<TextRange>,
        full_range: TextRange,
        kind: SymbolKind,
    ) -> NavigationTarget {
        NavigationTarget {
            file_id,
            name,
            kind: Some(kind),
            full_range,
            focus_range,
            container_name: None,
            description: None,
            alias: None,
        }
    }
}
