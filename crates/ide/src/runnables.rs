use crate::NavigationTarget;
use ide_db::helpers::visit_file_defs;
use ide_db::{RootDatabase, SymbolKind};
use lang::Semantics;
use lang::nameres::fq_named_element::ItemFQNameOwner;
use syntax::ast::HasItems;
use syntax::files::{InFile, InFileExt};
use syntax::{TextSize, ast};
use vfs::FileId;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Runnable {
    pub nav_item: NavigationTarget,
    pub test_path: String,
}

impl Runnable {
    pub fn label(&self) -> String {
        format!("test {}", self.test_path)
    }

    pub fn title(&self) -> String {
        let mut s = String::from("▶\u{fe0e} Run ");
        if self.nav_item.kind == Some(SymbolKind::Module) {
            s.push_str("Module Tests");
        } else {
            s.push_str("Test");
        }
        s
    }
}

pub(crate) fn runnables(db: &RootDatabase, file_id: FileId) -> Vec<Runnable> {
    let sema = Semantics::new(db, file_id);

    let mut res = Vec::new();
    visit_file_defs(&sema, file_id, &mut |named_item| {
        let runnable = match named_item {
            ast::NamedElement::Fun(fun) => runnable_fun(&sema, fun.in_file(file_id)),
            // ast::NamedElement::Module(module) => runnable_module(&sema, module.in_file(file_id)),
            _ => None,
        }?;
        res.push(runnable);
        Some(())
    });

    res.sort_by(cmp_runnables);
    res
}

pub(crate) fn runnable_module(
    sema: &Semantics<'_, RootDatabase>,
    module: InFile<ast::Module>,
) -> Option<Runnable> {
    if !module.value.functions().iter().any(|it| it.is_test()) {
        return None;
    }

    let mod_fq_name = module.fq_name(sema.db)?;
    let nav_item = NavigationTarget::from_named_item(module.map_into())?;

    let test_path = mod_fq_name.module_and_item_text();
    Some(Runnable { nav_item, test_path })
}

pub(crate) fn runnable_fun(
    sema: &Semantics<'_, RootDatabase>,
    fun: InFile<ast::Fun>,
) -> Option<Runnable> {
    if !fun.value.is_test() {
        return None;
    }

    let fq_name = fun.fq_name(sema.db)?;
    let nav_item = NavigationTarget::from_named_item(fun.map_into())?;

    let test_path = fq_name.module_and_item_text();
    Some(Runnable { nav_item, test_path })
}

fn cmp_runnables(run1: &Runnable, run2: &Runnable) -> std::cmp::Ordering {
    // full_range.start < focus_range.start < name, should give us a decent unique ordering
    run1.nav_item
        .full_range
        .start()
        .cmp(&run2.nav_item.full_range.start())
        .then_with(|| {
            let t_0 = || TextSize::from(0);
            run1.nav_item
                .focus_range
                .map_or_else(t_0, |it| it.start())
                .cmp(&run2.nav_item.focus_range.map_or_else(t_0, |it| it.start()))
        })
        .then_with(|| run1.nav_item.name.cmp(&run2.nav_item.name))
}
