use syntax::ast;
use syntax::ast::NamedItemScope;

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum UseItemType {
    Module,
    SelfModule,
    Item,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct UseItem {
    use_speck: ast::UseSpeck,
    use_alias: Option<ast::UseAlias>,
    name_or_alias: String,
    type_: UseItemType,
    scope: NamedItemScope,
}


