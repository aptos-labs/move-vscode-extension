use ide_db::text_edit::TextEdit;
use ide_db::{RootDatabase, SymbolKind};
use std::fmt;
use stdx::{impl_from, never};
use syntax::TextRange;

/// `CompletionItem` describes a single completion entity which expands to 1 or more entries in the
/// editor pop-up.
///
/// It is basically a POD with various properties. To construct a [`CompletionItem`],
/// use [`CompletionItemBuilder::new`] method and the [`CompletionItemBuilder`] struct.
#[derive(Clone)]
#[non_exhaustive]
pub struct CompletionItem {
    /// The primary label for the completion item.
    pub label: CompletionItemLabel,

    /// Range of identifier that is being completed.
    ///
    /// It should be used primarily for UI, but we also use this to convert
    /// generic TextEdit into LSP's completion edit (see conv.rs).
    ///
    /// `source_range` must contain the completion offset. `text_edit` should
    /// start with what `source_range` points to, or VSCode will filter out the
    /// completion silently.
    pub source_range: TextRange,

    /// What happens when user selects this item.
    ///
    /// Typically, replaces `source_range` with new identifier.
    pub text_edit: TextEdit,
    pub is_snippet: bool,

    /// What item (struct, function, etc) are we completing.
    pub kind: CompletionItemKind,

    /// Lookup is used to check if completion item indeed can complete current
    /// ident.
    ///
    /// That is, in `foo.bar$0` lookup of `abracadabra` will be accepted (it
    /// contains `bar` sub sequence), and `quux` will rejected.
    pub lookup: String,

    /// Additional info to show in the UI pop up.
    pub detail: Option<String>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CompletionItemLabel {
    /// The primary label for the completion item.
    pub primary: String,
    /// The left detail for the completion item, usually rendered right next to the primary label.
    pub detail_left: Option<String>,
    /// The right detail for the completion item, usually rendered right aligned at the end of the completion item.
    pub detail_right: Option<String>,
}

// We use custom debug for CompletionItem to make snapshot tests more readable.
impl fmt::Debug for CompletionItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("CompletionItem");
        s.field("label", &self.label.primary)
            .field("detail_left", &self.label.detail_left)
            .field("detail_right", &self.label.detail_right)
            .field("source_range", &self.source_range);
        if self.text_edit.len() == 1 {
            let text_change = self.text_edit.iter().next().unwrap();
            s.field("range", &text_change.range);
            s.field("new_text", &text_change.new_text);
        } else {
            s.field("text_edit", &self.text_edit);
        }
        s.field("kind", &self.kind);
        s.finish()
    }
}

/// The type of the completion item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompletionItemKind {
    SymbolKind(SymbolKind),
    Binding,
    BuiltinType,
    // InferredType,
    Keyword,
    // Snippet,
    UnresolvedReference,
    Expression,
}

impl_from!(SymbolKind for CompletionItemKind);

impl CompletionItemKind {
    pub fn tag(self) -> &'static str {
        match self {
            CompletionItemKind::SymbolKind(kind) => match kind {
                SymbolKind::Attribute => "at",
                SymbolKind::Const => "ct",
                SymbolKind::Enum => "en",
                SymbolKind::Field => "fd",
                SymbolKind::Function => "fn",
                SymbolKind::Label => "lb",
                SymbolKind::Local => "lc",
                SymbolKind::Method => "me",
                SymbolKind::Module => "md",
                SymbolKind::Struct => "st",
                SymbolKind::TypeParam => "tp",
                SymbolKind::ValueParam => "vp",
                SymbolKind::EnumVariant => "ev",
                SymbolKind::GlobalVariableDecl => "gv",
            },
            CompletionItemKind::Binding => "bn",
            CompletionItemKind::BuiltinType => "bt",
            CompletionItemKind::Keyword => "kw",
            CompletionItemKind::UnresolvedReference => "??",
            CompletionItemKind::Expression => "ex",
        }
    }
}

impl CompletionItem {
    pub(crate) fn new(
        kind: impl Into<CompletionItemKind>,
        source_range: TextRange,
        label: impl Into<String>,
    ) -> CompletionItemBuilder {
        let label = label.into();
        CompletionItemBuilder {
            source_range,
            label,
            insert_text: None,
            is_snippet: false,
            detail: None,
            lookup: None,
            kind: kind.into(),
            text_edit: None,
        }
    }

    /// What string is used for filtering.
    pub fn lookup(&self) -> &str {
        self.lookup.as_str()
    }
}

/// A helper to make `CompletionItem`s.
#[must_use]
#[derive(Clone)]
pub(crate) struct CompletionItemBuilder {
    source_range: TextRange,
    label: String,
    insert_text: Option<String>,
    is_snippet: bool,
    detail: Option<String>,
    lookup: Option<String>,
    kind: CompletionItemKind,
    text_edit: Option<TextEdit>,
}

impl CompletionItemBuilder {
    pub(crate) fn build(self, _db: &RootDatabase) -> CompletionItem {
        let label = self.label;
        let lookup = self.lookup.unwrap_or_else(|| label.clone());
        let insert_text = self.insert_text.unwrap_or_else(|| label.to_string());

        let detail_left = None;

        let text_edit = match self.text_edit {
            Some(it) => it,
            None => TextEdit::replace(self.source_range, insert_text),
        };

        CompletionItem {
            source_range: self.source_range,
            label: CompletionItemLabel {
                primary: label,
                detail_left,
                detail_right: self.detail.clone(),
            },
            kind: self.kind,
            text_edit,
            is_snippet: self.is_snippet,
            detail: self.detail,
            lookup,
        }
    }
    pub(crate) fn lookup_by(&mut self, lookup: impl Into<String>) -> &mut CompletionItemBuilder {
        self.lookup = Some(lookup.into());
        self
    }
    pub(crate) fn set_label(&mut self, label: impl Into<String>) -> &mut CompletionItemBuilder {
        self.label = label.into();
        self
    }
    pub(crate) fn insert_text(&mut self, insert_text: impl Into<String>) -> &mut CompletionItemBuilder {
        self.insert_text = Some(insert_text.into());
        self
    }
    pub(crate) fn insert_snippet(&mut self, snippet: impl Into<String>) -> &mut CompletionItemBuilder {
        self.is_snippet = true;
        self.insert_text(snippet)
    }
    pub(crate) fn text_edit(&mut self, edit: TextEdit) -> &mut CompletionItemBuilder {
        self.text_edit = Some(edit);
        self
    }
    pub(crate) fn detail(&mut self, detail: impl Into<String>) -> &mut CompletionItemBuilder {
        self.set_detail(Some(detail))
    }
    pub(crate) fn set_detail(
        &mut self,
        detail: Option<impl Into<String>>,
    ) -> &mut CompletionItemBuilder {
        self.detail = detail.map(Into::into);
        if let Some(detail) = &self.detail {
            if never!(detail.contains('\n'), "multiline detail:\n{}", detail) {
                self.detail = Some(detail.split('\n').next().unwrap().to_owned());
            }
        }
        self
    }
}
