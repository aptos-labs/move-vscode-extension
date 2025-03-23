use crate::ast::{support, AstChildren, FieldsOwner, HasAttrs, Stmt};
use crate::{ast, AstNode};

pub trait HasItems: AstNode {
    fn items(&self) -> AstChildren<ast::Item> {
        support::children(&self.syntax())
    }

    fn consts(&self) -> Vec<ast::Const> {
        self.items().into_iter().filter_map(|it| it.const_()).collect()
    }

    fn enums(&self) -> Vec<ast::Enum> {
        self.items().into_iter().filter_map(|it| it.enum_()).collect()
    }

    fn enum_variants(&self) -> Vec<ast::Variant> {
        self.enums().into_iter().flat_map(|e| e.variants()).collect()
    }

    fn non_test_functions(&self) -> Vec<ast::Fun> {
        self.functions()
            .into_iter()
            .filter(|fun| !fun.has_atom_attr("test"))
            .collect()
    }

    fn functions(&self) -> Vec<ast::Fun> {
        self.items().into_iter().filter_map(|it| it.fun()).collect()
    }

    fn structs(&self) -> Vec<ast::Struct> {
        self.items().into_iter().filter_map(|it| it.struct_()).collect()
    }

    fn all_item_specs(&self) -> Vec<ast::ItemSpec> {
        self.items().into_iter().filter_map(|it| it.item_spec()).collect()
    }

    fn module_item_specs(&self) -> Vec<ast::ItemSpec> {
        self.all_item_specs()
            .into_iter()
            .filter(|it| it.module_token().is_some())
            .collect()
    }

    fn item_specs(&self) -> Vec<ast::ItemSpec> {
        self.all_item_specs()
            .into_iter()
            .filter(|it| it.name_ref().is_some())
            .collect()
    }

    fn spec_functions(&self) -> Vec<ast::SpecFun> {
        self.items().into_iter().filter_map(|it| it.spec_fun()).collect()
    }

    fn spec_inline_functions(&self) -> Vec<ast::SpecInlineFun> {
        self.module_item_specs()
            .into_iter()
            .flat_map(|it| {
                it.spec_block()
                    .map(|it| it.spec_inline_functions())
                    .unwrap_or_default()
            })
            .collect()
    }

    fn schemas(&self) -> Vec<ast::Schema> {
        self.items().into_iter().filter_map(|it| it.schema()).collect()
    }

    fn tuple_structs(&self) -> Vec<ast::Struct> {
        self.structs()
            .into_iter()
            .filter(|s| s.tuple_field_list().is_some())
            .collect()
    }
}
