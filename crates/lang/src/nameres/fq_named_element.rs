use crate::HirDatabase;
use crate::nameres::address::Address;
use crate::node_ext::ModuleLangExt;
use crate::node_ext::item::ModuleItemExt;
use syntax::ast::NamedElement;
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, ast, match_ast};

pub enum ItemFQName {
    Module {
        address: Address,
        name: String,
    },
    Item {
        module_fq_name: Box<ItemFQName>,
        name: String,
    },
}

impl ItemFQName {
    pub fn identifier_text(&self) -> String {
        match self {
            ItemFQName::Module { address, name } => {
                let address_text = address.identifier_text();
                format!("{}::{}", address_text, name)
            }
            ItemFQName::Item { module_fq_name, name } => {
                let module_text = module_fq_name.identifier_text();
                format!("{}::{}", module_text, name)
            }
        }
    }

    pub fn address_identifier_text(&self) -> String {
        match self {
            ItemFQName::Module { address, .. } => address.identifier_text(),
            ItemFQName::Item { module_fq_name, .. } => module_fq_name.address_identifier_text(),
        }
    }

    pub fn module_identifier_text(&self) -> String {
        match self {
            ItemFQName::Module { .. } => self.identifier_text(),
            ItemFQName::Item { module_fq_name, .. } => module_fq_name.identifier_text(),
        }
    }
}

pub trait ItemFQNameOwner {
    fn fq_name(&self, db: &dyn HirDatabase) -> Option<ItemFQName>;
}

macro_rules! module_item_fq_name {
    ($module: expr, $it: expr) => {{
        let module_fq_name = $module.fq_name(db)?;
        let name = $it.name()?;
        Some(ItemFQName::Item {
            module_fq_name: Box::new(module_fq_name),
            name: name.as_string(),
        })
    }};
}

impl<T: AstNode> ItemFQNameOwner for InFile<T> {
    fn fq_name(&self, db: &dyn HirDatabase) -> Option<ItemFQName> {
        let it_file_id = self.file_id;
        let node = self.value.syntax();
        match_ast! {
            match node {
                ast::Module(it) => {
                    let address = it.address()?;
                    let name = it.name()?;
                    Some(ItemFQName::Module {
                        address,
                        name: name.as_string(),
                    })
                },
                ast::StructOrEnum(it) => {
                    let module_fq_name = it.module().in_file(it_file_id).fq_name(db)?;
                    let name = it.name()?;
                    Some(ItemFQName::Item {
                        module_fq_name: Box::new(module_fq_name),
                        name: name.as_string(),
                    })
                },
                ast::Const(it) => {
                    let module_fq_name = it.module()?.in_file(it_file_id).fq_name(db)?;
                    let name = it.name()?;
                    Some(ItemFQName::Item {
                        module_fq_name: Box::new(module_fq_name),
                        name: name.as_string(),
                    })
                },
                ast::AnyFun(it) => {
                    let module_fq_name = it.clone().in_file(it_file_id).module(db)?.fq_name(db)?;
                    let name = it.name()?;
                    Some(ItemFQName::Item {
                        module_fq_name: Box::new(module_fq_name),
                        name: name.as_string(),
                    })
                },
                _ => None
            }
        }
    }
}
