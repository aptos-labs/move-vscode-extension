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
    pub fn address(&self) -> &Address {
        match self {
            ItemFQName::Module { address, .. } => address,
            ItemFQName::Item { module_fq_name, .. } => module_fq_name.address(),
        }
    }

    pub fn fq_identifier_text(&self) -> String {
        match self {
            ItemFQName::Module { address, name } => {
                let address_text = address.identifier_text();
                format!("{}::{}", address_text, name)
            }
            ItemFQName::Item { module_fq_name, name } => {
                let module_text = module_fq_name.fq_identifier_text();
                format!("{}::{}", module_text, name)
            }
        }
    }

    pub fn module_and_item_text(&self) -> String {
        match self {
            ItemFQName::Module { address: _, name } => {
                name.to_string()
                // let address_text = address.identifier_text();
                // format!("{}::{}", address_text, name)
            }
            ItemFQName::Item { module_fq_name, name } => {
                let module_text = module_fq_name.module_and_item_text();
                format!("{}::{}", module_text, name)
            }
        }
    }

    pub fn address_identifier_text(&self) -> String {
        self.address().identifier_text()
        // match self {
        //     ItemFQName::Module { address, .. } => address.identifier_text(),
        //     ItemFQName::Item { module_fq_name, .. } => module_fq_name.address_identifier_text(),
        // }
    }

    pub fn module_identifier_text(&self) -> String {
        match self {
            ItemFQName::Module { .. } => self.fq_identifier_text(),
            ItemFQName::Item { module_fq_name, .. } => module_fq_name.fq_identifier_text(),
        }
    }

    pub fn name(&self) -> String {
        match self {
            ItemFQName::Module { name, .. } => name.to_string(),
            ItemFQName::Item { name, .. } => name.to_string(),
        }
    }
}

pub trait ItemFQNameOwner {
    fn fq_name(&self, db: &dyn HirDatabase) -> Option<ItemFQName>;
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
                ast::Schema(it) => {
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
