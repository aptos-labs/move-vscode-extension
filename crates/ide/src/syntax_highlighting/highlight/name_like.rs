use crate::syntax_highlighting::Highlight;
use crate::syntax_highlighting::tags::HlTag;
use ide_db::defs::{Definition, NameClass, NameRefClass};
use lang::Semantics;
use syntax::ast;

pub(crate) fn name_like(
    sema: &Semantics<'_>,
    // syntactic_name_ref_highlighting: bool,
    name_like: ast::NameLike,
) -> Option<Highlight> {
    let highlight = match name_like {
        ast::NameLike::NameRef(name_ref) => highlight_name_ref(sema, name_ref),
        ast::NameLike::Name(name) => highlight_name(name),
    };
    Some(highlight)
}

fn highlight_name(name: ast::Name) -> Highlight {
    let name_class = NameClass::classify(&name);
    match name_class {
        Some(NameClass::Definition(def)) => {
            let h = highlight_def(def) /*| HlMod::Definition*/;
            h
        }
        // Some(NameClass::PatFieldShorthand { field_ref, .. }) => {
        //     let mut h = HlTag::Symbol(SymbolKind::Field).into();
        //     if let hir::VariantDef::Union(_) = field_ref.parent_def(sema.db) {
        //         h |= HlMod::Unsafe;
        //     }
        //     h
        // }
        // None => highlight_name_by_syntax(name) | HlMod::Definition,
        None => HlTag::None.into(),
    }
}

fn highlight_name_ref(
    sema: &Semantics<'_>,
    // syntactic_name_ref_highlighting: bool,
    name_ref: ast::NameRef,
) -> Highlight {
    // if let Some(res) = highlight_method_call_by_name_ref(sema, krate, &name_ref) {
    //     return res;
    // }

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
        Definition::NamedItem(symbol_kind) => Highlight::new(HlTag::Symbol(symbol_kind)),
        Definition::BuiltinType => Highlight::new(HlTag::BuiltinType),
    }
}
