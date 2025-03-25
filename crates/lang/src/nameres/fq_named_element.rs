use crate::nameres::address::Address;
use crate::node_ext::ModuleLangExt;
use syntax::ast;
use syntax::ast::NamedElement;

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
}

pub trait FqNamedElement: NamedElement {
    fn fq_name(&self) -> Option<ItemFQName>;
}

impl FqNamedElement for ast::Module {
    fn fq_name(&self) -> Option<ItemFQName> {
        let address = self.address()?;
        let name = self.name()?;
        Some(ItemFQName::Module {
            address,
            name: name.as_string(),
        })
    }
}

impl FqNamedElement for ast::StructOrEnum {
    fn fq_name(&self) -> Option<ItemFQName> {
        let module_fq_name = self.module().fq_name()?;
        let name = self.name()?;
        Some(ItemFQName::Item {
            module_fq_name: Box::new(module_fq_name),
            name: name.as_string(),
        })
    }
}
