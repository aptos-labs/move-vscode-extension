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
    Prove { item_path: String },
}

impl Runnable {
    pub fn label(&self) -> String {
        match &self.kind {
            RunnableKind::Test { test_path } => format!("test {test_path}"),
            RunnableKind::Prove { item_path } => format!("prove {item_path}"),
        }
    }

    pub fn title(&self) -> String {
        match self.kind {
            RunnableKind::Test { .. } => {
                let mut s = String::from("▶\u{fe0e} Run ");
                if self.nav_item.kind == Some(SymbolKind::Module) {
                    s.push_str("Module Tests");
                } else {
                    s.push_str("Test");
                }
                s
            }
            RunnableKind::Prove { .. } => String::from("▶\u{fe0e} Check with Prover"),
        }
    }
}

pub(crate) fn runnables(db: &RootDatabase, file_id: FileId) -> Vec<Runnable> {
    let sema = Semantics::new(db, file_id);

    let mut res = Vec::new();
    visit_file_defs(&sema, file_id, &mut |named_item| {
        let runnable = match named_item {
            ast::NamedElement::Fun(fun) if fun.is_test() => {
                runnable_for_test_fun(&sema, fun.in_file(file_id))
            }
            _ => None,
        }?;
        res.push(runnable);
        Some(())
    });

    visit_item_specs(&sema, file_id, &mut |item_spec| {
        let item_spec_ref = item_spec.as_ref().and_then(|it| it.item_spec_ref())?;
        let fun = item_spec.item(sema.db)?.cast_into::<ast::Fun>()?;
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
    let fq_name = fun.fq_name(sema.db)?;
    let nav_item = NavigationTarget::from_named_item(fun.map_into())?;
    let test_path = fq_name.module_and_item_text();
    Some(Runnable {
        nav_item,
        kind: RunnableKind::Test { test_path },
    })
}

pub(crate) fn runnable_for_fun_item_spec(
    sema: &Semantics<'_, RootDatabase>,
    item_spec_ref: InFile<ast::ItemSpecRef>,
    fun: InFile<ast::Fun>,
) -> Option<Runnable> {
    let fq_name = fun.fq_name(sema.db)?;
    let nav_item = NavigationTarget::from_item_spec_ref(fq_name.name(), item_spec_ref)?;
    let item_path = fq_name.module_and_item_text();
    Some(Runnable {
        nav_item,
        kind: RunnableKind::Prove { item_path },
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
