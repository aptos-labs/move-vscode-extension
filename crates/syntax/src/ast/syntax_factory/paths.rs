use crate::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::ast::syntax_factory::{SyntaxFactory, ast_from_text, expr_item_from_text};
use crate::syntax_editor::SyntaxEditor;
use crate::syntax_editor::mapping::SyntaxMappingBuilder;
use crate::{AstNode, ast};
use itertools::Itertools;

impl SyntaxFactory {
    pub fn path_from_qualifier_and_name_ref(
        &self,
        qualifier: ast::Path,
        item_name_ref: ast::NameRef,
    ) -> ast::Path {
        let mut segments = qualifier.segments();
        segments.push(self.path_segment(item_name_ref));
        self.path_from_segments(segments)
    }

    pub fn item_path(
        &self,
        named_element: ast::NamedElement,
    ) -> Option<(ast::Path, Option<ast::NameRef>)> {
        let res = match named_element {
            ast::NamedElement::Module(module) => {
                let module_path = self.module_path(module.clone())?;
                ((module_path), None)
            }
            _ => {
                let module = named_element.syntax().containing_module()?;
                let module_path = self.module_path(module)?;

                let item_name = named_element.name()?.as_string();
                let item_name_ref = self.name_ref(&item_name);
                (module_path, Some(item_name_ref))
            }
        };
        Some(res)
    }

    fn module_path(&self, module: ast::Module) -> Option<ast::Path> {
        let address_ref = module.self_or_parent_address_ref()?;
        let address_ref_segment = self.path_segment_from_address_ref(address_ref);

        let module_name = module.name()?.as_string();
        let module_name_segment = self.path_segment_from_name(module_name);

        let module_path = self.path_from_segments(vec![address_ref_segment, module_name_segment]);
        Some(module_path)
    }

    pub fn path_from_segments(&self, segments: impl IntoIterator<Item = ast::PathSegment>) -> ast::Path {
        let segments = segments.into_iter().map(|it| it.syntax().clone()).join("::");
        expr_item_from_text(&segments)
    }

    pub fn path_segment(&self, name_ref: ast::NameRef) -> ast::PathSegment {
        let ast = ast_from_text::<ast::PathSegment>(&format!(
            "module 0x1::m {{ fun main() {{ let _ = {name_ref}; }}}}"
        ));

        if let Some(mut mapping) = self.mappings() {
            let mut builder = SyntaxMappingBuilder::new(ast.syntax().clone());
            builder.map_node(
                name_ref.syntax().clone(),
                ast.name_ref().unwrap().syntax().clone(),
            );
            builder.finish(&mut mapping);
        }

        ast
    }

    pub fn path_segment_from_text(&self, name: impl Into<String>) -> ast::PathSegment {
        let name = name.into();
        ast_from_text::<ast::PathSegment>(&format!(
            "module 0x1::m {{ fun main() {{ let _ = {name}; }}}}"
        ))
    }

    pub fn path_segment_from_name(&self, name: impl Into<String>) -> ast::PathSegment {
        let name = name.into();
        ast_from_text::<ast::PathSegment>(&format!(
            "module 0x1::m {{ fun main() {{ let _ = {name}; }}}}"
        ))
    }

    pub fn path_segment_from_address_ref(&self, address_ref: ast::AddressRef) -> ast::PathSegment {
        // let make = SyntaxFactory::new();
        let path_segment = match address_ref {
            ast::AddressRef::NamedAddress(named_address) => {
                self.path_segment_from_text(named_address.name())
            }
            ast::AddressRef::ValueAddress(value_address) => {
                self.path_segment_from_value_address(value_address.address_text())
            }
        };
        // editor.add_mappings(make.finish_with_mappings());
        path_segment
    }

    pub fn path_segment_from_value_address(&self, value_address: impl Into<String>) -> ast::PathSegment {
        let value_address = value_address.into();
        let path = ast_from_text::<ast::Path>(&format!(
            "module 0x1::m {{ const MY_CONST: {value_address}::my_path = 1; }}"
        ));
        let qualifier = path.qualifier().unwrap();
        qualifier
            .path_address()
            .unwrap()
            .syntax()
            .parent_of_type()
            .unwrap()
    }
}
