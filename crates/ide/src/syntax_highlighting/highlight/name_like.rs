use crate::syntax_highlighting::Highlight;
use crate::syntax_highlighting::tags::HlTag;
use ide_db::RootDatabase;
use ide_db::defs::{Definition, NameClass, NameRefClass};
use lang::Semantics;
use syntax::ast;

pub(crate) fn name_like(
    sema: &Semantics<'_, RootDatabase>,
    name_like: ast::NameLike,
) -> Option<Highlight> {
    let highlight = match name_like {
        ast::NameLike::NameRef(name_ref) => highlight_name_ref(sema, name_ref),
        ast::NameLike::Name(name) => highlight_name(sema, name),
    };
    Some(highlight)
}

fn highlight_name(sema: &Semantics<'_, RootDatabase>, name: ast::Name) -> Highlight {
    let name_class = NameClass::classify(sema, name);
    match name_class {
        Some(NameClass::Definition(def)) => {
            let h = highlight_def(def) /*| HlMod::Definition*/;
            h
        }
        // Some(NameClass::PatFieldShorthand { field_ref, .. }) => {
        //     let mut h = HlTag::Symbol(SymbolKind::Field).into();
        //     h
        // }
        // None => highlight_name_by_syntax(name) | HlMod::Definition,
        None => HlTag::None.into(),
    }
}

fn highlight_name_ref(sema: &Semantics<'_, RootDatabase>, name_ref: ast::NameRef) -> Highlight {
    let name_ref_class = match NameRefClass::classify(sema, &name_ref) {
        Some(name_kind) => name_kind,
        None => return HlTag::UnresolvedReference.into(),
    };
    let h = match name_ref_class {
        NameRefClass::Definition(def) => highlight_def(def),
    };

    h
}

pub(crate) fn highlight_def(def: Definition) -> Highlight {
    match def {
        Definition::NamedItem(symbol_kind, _) => Highlight::new(HlTag::Symbol(symbol_kind)),
        Definition::BuiltinType => Highlight::new(HlTag::BuiltinType),
    }
}
