// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::NavigationTarget;
use ide_db::helpers::{visit_file_defs, visit_item_specs};
use ide_db::{RootDatabase, SymbolKind};
use lang::Semantics;
use lang::nameres::fq_named_element::ItemFQNameOwner;
use lang::node_ext::item_spec::ItemSpecExt;
use syntax::ast::HasItems;
use syntax::files::{InFile, InFileExt};
use syntax::{TextSize, ast};
use vfs::FileId;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Runnable {
    pub nav_item: NavigationTarget,
    pub kind: RunnableKind,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum RunnableKind {
    Test { test_path: String },
    ProveFun { only: String },
    ProveModule { filter: String },
}

impl Runnable {
    pub fn label(&self) -> String {
        match &self.kind {
            RunnableKind::Test { test_path } => format!("test {test_path}"),
            RunnableKind::ProveFun { only } => format!("prove fun {only}"),
            RunnableKind::ProveModule { filter } => format!("prove mod {filter}"),
        }
    }

    pub fn title(&self) -> String {
        match self.kind {
            RunnableKind::Test { .. } => {
                let mut s = String::from("▶\u{fe0e} Run ");
                if self.nav_item.kind == Some(SymbolKind::Module) {
                    s.push_str("Tests");
                } else {
                    s.push_str("Test");
                }
                s
            }
            RunnableKind::ProveFun { .. } | RunnableKind::ProveModule { .. } => {
                String::from("▶\u{fe0e} Check with Prover")
            }
        }
    }
}

pub(crate) fn runnables(db: &RootDatabase, file_id: FileId) -> Vec<Runnable> {
    let sema = Semantics::new(db, file_id);

    let mut res = Vec::new();
    visit_file_defs(&sema, file_id, &mut |named_item| {
        let runnable = match named_item {
            ast::NamedElement::Fun(fun) => runnable_for_test_fun(&sema, fun.in_file(file_id)),
            ast::NamedElement::Module(module) => {
                runnable_for_module_with_test_funs(&sema, module.in_file(file_id))
            }
            _ => None,
        }?;
        res.push(runnable);
        Some(())
    });

    let file = sema.parse(file_id);
    for module_spec in file.module_specs() {
        if let Some(runnable) = runnable_for_module_spec(&sema, module_spec.in_file(file_id)) {
            res.push(runnable);
        }
    }

    visit_item_specs(&sema, file_id, &mut |item_spec| {
        let item_spec_ref = item_spec.as_ref().and_then(|it| it.item_spec_ref())?;
        let fun = item_spec.item(sema.db)?.and_then(|it| it.fun())?;
        if let Some(runnable) = runnable_for_fun_item_spec(&sema, item_spec_ref, fun) {
            res.push(runnable);
        }
        Some(())
    });

    res.sort_by(cmp_runnables);
    res
}

pub(crate) fn runnable_for_test_fun(
    sema: &Semantics<'_, RootDatabase>,
    fun: InFile<ast::Fun>,
) -> Option<Runnable> {
    if !fun.value.is_test() {
        return None;
    }
    let fq_name = fun.fq_name(sema.db)?;
    let nav_item = NavigationTarget::from_named_item(fun)?;
    let test_path = fq_name.module_and_item_text();
    Some(Runnable {
        nav_item,
        kind: RunnableKind::Test { test_path },
    })
}

pub(crate) fn runnable_for_module_with_test_funs(
    _sema: &Semantics<'_, RootDatabase>,
    module: InFile<ast::Module>,
) -> Option<Runnable> {
    if !module.value.functions().iter().any(|it| it.is_test()) {
        return None;
    }
    let nav_item = NavigationTarget::from_named_item(module)?;
    let test_path = format!("{}::", &nav_item.name);
    Some(Runnable {
        nav_item,
        kind: RunnableKind::Test { test_path },
    })
}

pub(crate) fn runnable_for_module_spec(
    sema: &Semantics<'_, RootDatabase>,
    module_spec: InFile<ast::ModuleSpec>,
) -> Option<Runnable> {
    let nav_item = NavigationTarget::from_module_spec(sema, module_spec)?;
    let module_name = nav_item.name.clone();
    Some(Runnable {
        nav_item,
        kind: RunnableKind::ProveModule { filter: module_name },
    })
}

pub(crate) fn runnable_for_fun_item_spec(
    sema: &Semantics<'_, RootDatabase>,
    item_spec_ref: InFile<ast::ItemSpecRef>,
    fun: InFile<ast::Fun>,
) -> Option<Runnable> {
    let fq_name = fun.fq_name(sema.db)?;
    let nav_item = NavigationTarget::from_item_spec_ref(fq_name.name(), item_spec_ref)?;
    let fq_item_path = fq_name.module_and_item_text();
    Some(Runnable {
        nav_item,
        kind: RunnableKind::ProveFun { only: fq_item_path },
    })
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
