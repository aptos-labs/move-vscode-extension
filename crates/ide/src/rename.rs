use crate::RangeInfo;
use ide_db::defs::{Definition, NameClass, NameRefClass};
use ide_db::rename::{IdentifierKind, RenameError, bail, format_err};
use ide_db::source_change::SourceChange;
use ide_db::{RootDatabase, rename};
use lang::Semantics;
use syntax::files::{FilePosition, InFile};
use syntax::{AstNode, SyntaxNode, ast};

type RenameResult<T> = Result<T, RenameError>;

/// Prepares a rename. The sole job of this function is to return the TextRange of the thing that is
/// being targeted for a rename.
pub(crate) fn prepare_rename(db: &RootDatabase, position: FilePosition) -> RenameResult<RangeInfo<()>> {
    let sema = Semantics::new(db, position.file_id);
    let source_file = sema.parse(position.file_id);
    let syntax = source_file.syntax();

    let named_element = find_definition(&sema, syntax, position)?;
    let name = named_element
        .value
        .name()
        .ok_or_else(|| format_err!("No references found at position"))?;

    Ok(RangeInfo::new(name.syntax().text_range(), ()))
}

pub(crate) fn rename(
    db: &RootDatabase,
    position: FilePosition,
    new_name: &str,
) -> RenameResult<SourceChange> {
    let sema = Semantics::new(db, position.file_id);
    let source_file = sema.parse(position.file_id);
    let syntax = source_file.syntax();

    let _ = IdentifierKind::classify(new_name)?;

    let named_element = find_definition(&sema, syntax, position)?;
    let change = rename::rename_named_element(&sema, named_element, new_name)?;

    Ok(change)
}

fn find_definition(
    sema: &Semantics<'_, RootDatabase>,
    syntax: &SyntaxNode,
    FilePosition { file_id: _, offset }: FilePosition,
) -> RenameResult<InFile<ast::NamedElement>> {
    let name_like = sema
        .find_namelike_at_offset(syntax, offset)
        .ok_or_else(|| format_err!("No references found at position"))?;

    let def = match &name_like {
        // renaming aliases would rename the item being aliased as the HIR doesn't track aliases yet
        ast::NameLike::Name(name)
            if name
                .syntax()
                .parent()
                .is_some_and(|it| ast::UseAlias::can_cast(it.kind())) =>
        {
            bail!("Renaming aliases is currently unsupported")
        }
        ast::NameLike::Name(name) => {
            let name_class = NameClass::classify(sema, name.clone())
                .ok_or_else(|| format_err!("No references found at position"))?;
            match name_class {
                NameClass::Definition(Definition::NamedItem(_, named_item))
                | NameClass::ConstReference(Definition::NamedItem(_, named_item)) => named_item,
                NameClass::PatFieldShorthand { ident_pat, .. } => ident_pat.map(|it| it.into()),
                NameClass::ItemSpecFunctionParam { fun_param_ident_pat, .. } => {
                    fun_param_ident_pat.map(|it| it.into())
                }
                _ => {
                    bail!("No references found at position");
                }
            }
        }
        ast::NameLike::NameRef(name_ref) => {
            let name_ref_class = NameRefClass::classify(sema, name_ref)
                .ok_or_else(|| format_err!("No references found at position"))?;
            let def = match name_ref_class {
                NameRefClass::Definition(Definition::NamedItem(_, named_item)) => named_item,
                NameRefClass::Definition(Definition::BuiltinType) => {
                    bail!("Cannot rename built-in type.");
                }
                NameRefClass::FieldShorthand { ident_pat, named_field: _ } => {
                    ident_pat.map(|it| it.into())
                }
            };

            // if the name differs from the definitions name it has to be an alias
            if def
                .value
                .name()
                .is_some_and(|it| it.as_string() != name_ref.text())
            {
                bail!("Renaming aliases is currently unsupported");
            }

            def
        }
    };

    Ok(def)
}
