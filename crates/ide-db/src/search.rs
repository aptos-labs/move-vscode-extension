use crate::RootDatabase;
use crate::defs::{Definition, NameClass, NameRefClass};
use base_db::SourceDatabase;
use base_db::package_root::PackageId;
use lang::nameres::node_ext::ModuleResolutionExt;
use lang::{Semantics, hir_db};
use memchr::memmem::Finder;
use std::cell::LazyCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::{iter, mem};
use syntax::ast::IdentPatKind;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::files::{FileRange, InFile};
use syntax::{AstNode, SyntaxElement, SyntaxNode, TextRange, TextSize, ast};
use vfs::FileId;

#[derive(Debug, Default, Clone)]
pub struct UsageSearchResult {
    pub references: HashMap<FileId, Vec<FileReference>>,
}

impl UsageSearchResult {
    pub fn is_empty(&self) -> bool {
        self.references.is_empty()
    }

    pub fn len(&self) -> usize {
        self.references.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (FileId, &[FileReference])> + '_ {
        self.references.iter().map(|(&file_id, refs)| (file_id, &**refs))
    }

    pub fn file_ranges(&self) -> impl Iterator<Item = FileRange> + '_ {
        self.references.iter().flat_map(|(&file_id, refs)| {
            refs.iter()
                .map(move |&FileReference { range, .. }| FileRange { file_id, range })
        })
    }
}

impl IntoIterator for UsageSearchResult {
    type Item = (FileId, Vec<FileReference>);
    type IntoIter = <HashMap<FileId, Vec<FileReference>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.references.into_iter()
    }
}

#[derive(Debug, Clone)]
pub struct FileReference {
    /// The range of the reference in the original file
    pub range: TextRange,
    /// The node of the reference in the file
    pub name: FileReferenceNode,
}

#[derive(Debug, Clone)]
pub enum FileReferenceNode {
    Name(ast::Name),
    NameRef(ast::NameRef),
}

impl FileReferenceNode {
    pub fn text_range(&self) -> TextRange {
        match self {
            FileReferenceNode::Name(it) => it.syntax().text_range(),
            FileReferenceNode::NameRef(it) => it.syntax().text_range(),
        }
    }
    pub fn syntax(&self) -> SyntaxElement {
        match self {
            FileReferenceNode::Name(it) => it.syntax().clone().into(),
            FileReferenceNode::NameRef(it) => it.syntax().clone().into(),
        }
    }
    pub fn into_name_like(self) -> Option<ast::NameLike> {
        match self {
            FileReferenceNode::Name(it) => Some(ast::NameLike::Name(it)),
            FileReferenceNode::NameRef(it) => Some(ast::NameLike::NameRef(it)),
        }
    }
    pub fn as_name_ref(&self) -> Option<&ast::NameRef> {
        match self {
            FileReferenceNode::NameRef(name_ref) => Some(name_ref),
            _ => None,
        }
    }
    pub fn text(&self) -> syntax::TokenText<'_> {
        match self {
            FileReferenceNode::Name(name) => name.text(),
            FileReferenceNode::NameRef(name_ref) => name_ref.text(),
        }
    }
}

pub fn item_search_scope(db: &RootDatabase, named_item: &InFile<ast::NamedElement>) -> SearchScope {
    let _p = tracing::info_span!("item_search_scope").entered();

    let (file_id, _named_item) = named_item.unpack_ref();
    let package_id = db.file_package_id(file_id);

    if let Some(ident_pat) = named_item.cast_into_ref::<ast::IdentPat>() {
        if let Some(search_scope) = ident_pat_search_scope(db, ident_pat) {
            return search_scope;
        }
    }

    // accessible only in the module it was defined in (and in specs)
    if let Some(named_field) = named_item.cast_into_ref::<ast::NamedField>() {
        if let Some(containing_module) = named_field.and_then(|it| it.syntax().containing_module()) {
            return SearchScope::module_and_module_spec(db, containing_module);
        }
    }

    SearchScope::reverse_dependencies(db, package_id)
}

fn ident_pat_search_scope(db: &RootDatabase, ident_pat: InFile<ast::IdentPat>) -> Option<SearchScope> {
    let module = ident_pat.and_then_ref(|it| it.syntax().containing_module())?;
    let owner_kind = ident_pat.value.owner()?;
    match owner_kind {
        IdentPatKind::Param(_) => Some(SearchScope::module_and_module_spec(db, module)),
        IdentPatKind::LambdaParam(lambda_param) => {
            let lambda_expr = lambda_param.lambda_expr();
            Some(SearchScope::file_range(FileRange {
                file_id: ident_pat.file_id,
                range: lambda_expr.syntax().text_range(),
            }))
        }
        IdentPatKind::LetStmt(_) => {
            let fun = ident_pat.and_then(|it| it.syntax().containing_function())?;
            Some(SearchScope::file_range(fun.syntax().file_range()))
        }
    }
}

pub fn item_usages<'a>(
    sema: &'a Semantics<'a, RootDatabase>,
    named_item: InFile<ast::NamedElement>,
) -> FindUsages<'a> {
    FindUsages {
        named_item,
        sema,
        scope: None,
    }
}

/// Generally, `search_scope` returns files that might contain references for the element.
/// For `pub(crate)` things it's a crate, for `pub` things it's a crate and dependant crates.
/// In some cases, the location of the references is known to within a `TextRange`,
/// e.g. for things like local variables.
#[derive(Clone, Debug)]
pub struct SearchScope {
    entries: HashMap<FileId, Option<TextRange>>,
}

impl SearchScope {
    fn new(entries: HashMap<FileId, Option<TextRange>>) -> SearchScope {
        SearchScope { entries }
    }

    // /// Build a search scope spanning the entire crate graph of files.
    // pub fn all_packages(db: &RootDatabase) -> SearchScope {
    //     let mut entries = HashMap::default();
    //     let all_package_ids = db.all_package_ids();
    //     for package_id in all_package_ids.data(db) {
    //         let source_file_ids = hir_db::source_file_ids_in_package(db, package_id);
    //         entries.extend(source_file_ids.iter().map(|file_id| (*file_id, None)));
    //     }
    //     SearchScope { entries }
    // }

    /// Build a search scope spanning all the reverse dependencies of the given crate.
    pub fn reverse_dependencies(db: &RootDatabase, of: PackageId) -> SearchScope {
        let mut entries = HashMap::default();
        for rev_dep in hir_db::reverse_transitive_dep_package_ids(db, of) {
            let file_ids = hir_db::source_file_ids_in_package(db, rev_dep);
            entries.extend(file_ids.iter().map(|file_id| (*file_id, None)));
        }
        SearchScope { entries }
    }

    /// Build a search scope spanning the given crate.
    pub fn package(db: &RootDatabase, of: PackageId) -> SearchScope {
        SearchScope {
            entries: hir_db::source_file_ids_in_package(db, of)
                .iter()
                .map(|file_id| (*file_id, None))
                .collect(),
        }
    }

    pub fn module_and_module_spec(db: &RootDatabase, module: InFile<ast::Module>) -> SearchScope {
        fn node_search_scope(item: InFile<SyntaxNode>) -> (FileId, Option<TextRange>) {
            let (file_id, item) = item.unpack();
            (file_id, Some(item.text_range()))
        }

        let mut entries = vec![(module.file_id, None)];
        for module_spec in module.related_module_specs(db) {
            entries.push((module_spec.file_id, None));
        }

        SearchScope::new(entries.into_iter().collect())
    }

    /// Build an empty search scope.
    pub fn empty() -> SearchScope {
        SearchScope::new(HashMap::default())
    }

    /// Build a empty search scope spanning the given file.
    pub fn single_file(file: FileId) -> SearchScope {
        SearchScope::new(iter::once((file, None)).collect())
    }

    /// Build a empty search scope spanning the text range of the given file.
    pub fn file_range(range: FileRange) -> SearchScope {
        SearchScope::new(iter::once((range.file_id, Some(range.range))).collect())
    }

    /// Build a empty search scope spanning the given files.
    pub fn files(files: &[FileId]) -> SearchScope {
        SearchScope::new(files.iter().map(|f| (*f, None)).collect())
    }

    pub fn intersection(&self, other: &SearchScope) -> SearchScope {
        let (mut small, mut large) = (&self.entries, &other.entries);
        if small.len() > large.len() {
            mem::swap(&mut small, &mut large)
        }

        let intersect_ranges =
            |r1: Option<TextRange>, r2: Option<TextRange>| -> Option<Option<TextRange>> {
                match (r1, r2) {
                    (None, r) | (r, None) => Some(r),
                    (Some(r1), Some(r2)) => r1.intersect(r2).map(Some),
                }
            };
        let res = small
            .iter()
            .filter_map(|(&file_id, &r1)| {
                let &r2 = large.get(&file_id)?;
                let r = intersect_ranges(r1, r2)?;
                Some((file_id, r))
            })
            .collect();

        SearchScope::new(res)
    }
}

impl IntoIterator for SearchScope {
    type Item = (FileId, Option<TextRange>);
    type IntoIter = std::collections::hash_map::IntoIter<FileId, Option<TextRange>>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

#[derive(Clone)]
pub struct FindUsages<'a> {
    named_item: InFile<ast::NamedElement>,
    // rename: Option<&'a Rename>, - alias
    sema: &'a Semantics<'a, RootDatabase>,
    scope: Option<SearchScope>,
}

impl<'a> FindUsages<'a> {
    /// Limit the search to a given [`SearchScope`].
    pub fn in_scope(self, scope: SearchScope) -> Self {
        self.set_scope(Some(scope))
    }

    /// Limit the search to a given [`SearchScope`].
    pub fn set_scope(mut self, scope: Option<SearchScope>) -> Self {
        assert!(self.scope.is_none());
        self.scope = scope;
        self
    }

    pub fn at_least_one(&self) -> bool {
        let mut found = false;
        self.search(&mut |_, _| {
            found = true;
            true
        });
        found
    }

    pub fn fetch_all(self) -> UsageSearchResult {
        let mut res = UsageSearchResult::default();
        self.search(&mut |file_id, reference| {
            res.references.entry(file_id).or_default().push(reference);
            false
        });
        res
    }

    fn scope_files<'b>(
        db: &'b RootDatabase,
        scope: &'b SearchScope,
    ) -> impl Iterator<Item = (Arc<str>, FileId, TextRange)> + 'b {
        scope.entries.iter().map(|(&file_id, &search_range)| {
            let text = db.file_text(file_id).text(db);
            let search_range = search_range.unwrap_or_else(|| TextRange::up_to(TextSize::of(&*text)));

            (text, file_id, search_range)
        })
    }

    fn match_offsets<'b>(
        text: &'b str,
        finder: &'b Finder<'b>,
        search_range: TextRange,
    ) -> impl Iterator<Item = TextSize> + 'b {
        finder.find_iter(text.as_bytes()).filter_map(move |idx| {
            let offset: TextSize = idx.try_into().unwrap();
            if !search_range.contains_inclusive(offset) {
                return None;
            }
            // If this is not a word boundary, that means this is only part of an identifier,
            // so it can't be what we're looking for.
            // This speeds up short identifiers significantly.
            if text[..idx]
                .chars()
                .next_back()
                .is_some_and(|ch| matches!(ch, 'A'..='Z' | 'a'..='z' | '_'))
                || text[idx + finder.needle().len()..]
                    .chars()
                    .next()
                    .is_some_and(|ch| matches!(ch, 'A'..='Z' | 'a'..='z' | '_' | '0'..='9'))
            {
                return None;
            }
            Some(offset)
        })
    }

    fn find_nodes<'b>(
        name: &str,
        node: &SyntaxNode,
        offset: TextSize,
    ) -> impl Iterator<Item = SyntaxNode> + 'b {
        node.token_at_offset(offset)
            .find(|it| it.text() == name)
            .into_iter()
            .filter_map(|it| it.parent())
    }

    pub fn search(&self, sink: &mut dyn FnMut(FileId, FileReference) -> bool) {
        let _p = tracing::info_span!("FindUsages:search").entered();
        let sema = self.sema;

        let search_scope = {
            let base = item_search_scope(sema.db, &self.named_item);
            match &self.scope {
                None => base,
                Some(scope) => base.intersection(scope),
            }
        };

        let name = self.named_item.value.name();
        let name = match &name {
            Some(s) => s.as_string(),
            None => return,
        };

        let finder = &Finder::new(&name);
        for (text, file_id, search_range) in Self::scope_files(sema.db, &search_scope) {
            let tree = LazyCell::new(move || sema.parse(file_id).syntax().clone());

            for offset in Self::match_offsets(&text, finder, search_range) {
                let nodes_at_offset = Self::find_nodes(&name, &tree, offset);
                for name_like in nodes_at_offset.filter_map(ast::NameLike::cast) {
                    if match name_like {
                        ast::NameLike::NameRef(name_ref) => self.found_name_ref(&name_ref, sink),
                        ast::NameLike::Name(name) => self.found_name(&name, sink),
                    } {
                        return;
                    }
                }
            }
        }
    }

    fn found_name_ref(
        &self,
        name_ref: &ast::NameRef,
        sink: &mut dyn FnMut(FileId, FileReference) -> bool,
    ) -> bool {
        let name_ref_class = NameRefClass::classify(self.sema, name_ref);
        match name_ref_class {
            Some(NameRefClass::Definition(Definition::NamedItem(_, named_item)))
                if self.named_item == named_item =>
            {
                let FileRange { file_id, range } = self.sema.file_range(name_ref.syntax());
                let reference = FileReference {
                    range,
                    name: FileReferenceNode::NameRef(name_ref.clone()),
                };
                sink(file_id, reference)
            }
            Some(NameRefClass::FieldShorthand { ident_pat, named_field })
                if self.named_item == ident_pat.clone().map_into()
                    || self.named_item == named_field.clone().map_into() =>
            {
                let FileRange { file_id, range } = self.sema.file_range(name_ref.syntax());
                let reference = FileReference {
                    range,
                    name: FileReferenceNode::NameRef(name_ref.clone()),
                };
                sink(file_id, reference)
            }
            _ => false,
        }
    }

    fn found_name(&self, name: &ast::Name, sink: &mut dyn FnMut(FileId, FileReference) -> bool) -> bool {
        match NameClass::classify(self.sema, name.clone()) {
            Some(NameClass::ItemSpecFunctionParam {
                spec_ident_pat,
                fun_param_ident_pat,
            }) if self.named_item == fun_param_ident_pat.clone().map_into() => {
                let reference = FileReference {
                    range: name.syntax().text_range(),
                    name: FileReferenceNode::Name(name.clone()),
                };
                sink(spec_ident_pat.file_id, reference)
            }
            Some(NameClass::PatFieldShorthand { ident_pat, named_field })
                if self.named_item == named_field.clone().map_into() =>
            {
                let reference = FileReference {
                    range: name.syntax().text_range(),
                    name: FileReferenceNode::Name(name.clone()),
                };
                sink(ident_pat.file_id, reference)
            }
            _ => false,
        }
    }
}
