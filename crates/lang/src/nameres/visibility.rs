use syntax::ast;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VisibilityLevel {
    Friend(Vec<ast::Module>),
    Package, /*(PackageId)*/
    Script,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Visibility {
    Public,
    Restricted(VisibilityLevel),
    Private,
}
