use crate::search::{FileReference, FileReferenceNode};
use crate::source_change::SourceChange;
use crate::text_edit::{TextEdit, TextEditBuilder};
use crate::{RootDatabase, search};
use base_db::SourceDatabase;
use lang::Semantics;
use std::fmt;
use syntax::files::InFile;
use syntax::{AstNode, SyntaxKind, TextRange, ast};

pub type Result<T, E = RenameError> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct RenameError(pub String);

impl fmt::Display for RenameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[macro_export]
macro_rules! _format_err {
    ($fmt:expr) => { RenameError(format!($fmt)) };
    ($fmt:expr, $($arg:tt)+) => { RenameError(format!($fmt, $($arg)+)) }
}

pub use _format_err as format_err;

#[macro_export]
macro_rules! _bail {
    ($($tokens:tt)*) => { return Err(format_err!($($tokens)*)) }
}
pub use _bail as bail;

pub fn rename_named_element<'db>(
    sema: &'db Semantics<'db, RootDatabase>,
    named_element: InFile<ast::NamedElement>,
    new_name: &str,
) -> Result<SourceChange> {
    if sema.is_builtins_file(named_element.file_id) {
        bail!("Cannot rename a builtin item.");
    }
    let package_id = sema.db.file_package_id(named_element.file_id);
    if sema.is_library(package_id) {
        bail!("Cannot rename a non-local definition");
    }

    let mut source_change = SourceChange::default();

    let usages = search::item_usages(sema, named_element.clone()).fetch_all();

    for (file_id, references) in usages.iter() {
        let text_edit = source_edit_from_references(references, named_element.value.clone(), new_name);
        source_change.insert_source_edit(file_id, text_edit);
    }

    // This needs to come after the references edits, because we change the annotation of existing edits
    // if a conflict is detected.
    let (file_id, named_element) = named_element.unpack();

    let edit = source_edit_from_def(named_element, new_name)?;
    source_change.insert_source_edit(file_id, edit);

    Ok(source_change)
}

pub fn source_edit_from_references(
    references: &[FileReference],
    named_element: ast::NamedElement,
    new_name: &str,
) -> TextEdit {
    let mut edit = TextEdit::builder();
    for &FileReference { range, ref name, .. } in references {
        let has_emitted_edit = match name {
            FileReferenceNode::NameRef(name_ref) => {
                source_edit_from_name_ref(&mut edit, name_ref, &new_name, &named_element)
            }
            FileReferenceNode::Name(name) => source_edit_from_name(&mut edit, name, &new_name),
        };
        if !has_emitted_edit {
            edit.replace(range, new_name.to_string());
        }
    }

    edit.finish()
}

fn source_edit_from_name(
    edit: &mut TextEditBuilder,
    name: &ast::Name,
    new_name: &dyn fmt::Display,
) -> bool {
    if let Some(struct_pat_field) = ast::StructPatField::for_field_name(name) {
        let ident_pat = struct_pat_field.ident_pat().unwrap();
        // Foo { field } -> Foo { new_name: field }
        //      ^ insert `new_name: `
        edit.insert(ident_pat.syntax().text_range().start(), format!("{new_name}: "));
        return true;
    }
    false
}

fn source_edit_from_name_ref(
    edit: &mut TextEditBuilder,
    name_ref: &ast::NameRef,
    new_name: &dyn fmt::Display,
    named_element: &ast::NamedElement,
) -> bool {
    if let Some(struct_lit_field_kind) = name_ref.try_into_struct_lit_field() {
        match struct_lit_field_kind {
            ast::StructLitFieldKind::Full {
                struct_field: _,
                name_ref: field_name_ref,
                expr,
            } => {
                // field: init-expr, check if we can use a field init shorthand
                if let Some(expr) = expr {
                    let new_name = new_name.to_string();
                    if &field_name_ref == name_ref && expr.syntax().text().to_string() == new_name {
                        // Foo { field: local } -> Foo { local }
                        //       ^^^^^^^ delete this
                        let name_start = field_name_ref.syntax().text_range().start();
                        let expr_start = expr.syntax().text_range().start();
                        edit.delete(TextRange::new(name_start, expr_start));
                        return true;
                    }
                }
            }
            ast::StructLitFieldKind::Shorthand { .. } => {
                match named_element {
                    ast::NamedElement::NamedField(_) => {
                        // Foo { field } -> Foo { new_name: field }
                        //       ^ insert `new_name: `
                        let offset = name_ref.syntax().text_range().start();
                        edit.insert(offset, format!("{new_name}: "));
                        return true;
                    }
                    ast::NamedElement::IdentPat(_) => {
                        // Foo { field } -> Foo { field: new_name }
                        //            ^ insert `: new_name`
                        let offset = name_ref.syntax().text_range().end();
                        edit.insert(offset, format!(": {new_name}"));
                        return true;
                    }
                    _ => (),
                }
            }
        }
    } else if let Some(struct_pat_field) = ast::StructPatField::for_field_name_ref(name_ref) {
        let field_name_ref = struct_pat_field.name_ref();
        let field_ident_pat = struct_pat_field.ident_pat();
        match (field_name_ref, field_ident_pat) {
            // field: rename
            (Some(field_name), Some(ident_pat)) if field_name == *name_ref => {
                // field name is being renamed
                if let Some(name) = ident_pat.name() {
                    let new_name = new_name.to_string();
                    if name.text() == new_name {
                        // Foo { field: local } -> Foo { field }
                        //       ^^^^^^^ delete this
                        //              ^^^^^ replace this with `field`

                        // same names, we can use a shorthand here instead/
                        // we do not want to erase attributes hence this range start
                        let s = field_name.syntax().text_range().start();
                        let e = ident_pat.syntax().text_range().start();
                        edit.delete(TextRange::new(s, e));
                        edit.replace(name.syntax().text_range(), new_name);
                        return true;
                    }
                }
            }
            _ => (),
        }
    }
    false
}

fn source_edit_from_def(named_element: ast::NamedElement, new_name: &str) -> Result<TextEdit> {
    let mut edit = TextEdit::builder();

    let old_name_range = named_element.name().unwrap().syntax().text_range();
    edit.replace(old_name_range, new_name.to_string());
    Ok(edit.finish())
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum IdentifierKind {
    Ident,
    Label,
    Underscore,
}

impl IdentifierKind {
    pub fn classify(new_name: &str) -> Result<IdentifierKind> {
        let token = syntax::parse_single_token(new_name);
        match token {
            None => {
                bail!("Invalid name `{}`: not an identifier", new_name)
            }
            Some(token) => match token.kind {
                SyntaxKind::IDENT => Ok(IdentifierKind::Ident),
                SyntaxKind::QUOTE_IDENT => Ok(IdentifierKind::Label),
                SyntaxKind::UNDERSCORE => Ok(IdentifierKind::Underscore),
                _ if SyntaxKind::from_keyword(new_name).is_some() => {
                    bail!("Invalid name `{}`: cannot rename to a keyword", new_name)
                }
                _ => bail!("Invalid name `{}`: not an identifier", new_name),
            },
        }
    }
}
