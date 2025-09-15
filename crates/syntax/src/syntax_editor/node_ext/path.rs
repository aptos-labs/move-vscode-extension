use crate::ast::node_ext::syntax_element::SyntaxElementExt;
use crate::ast::syntax_factory::SyntaxFactory;
use crate::syntax_editor::SyntaxEditor;
use crate::{AstNode, ast};

impl ast::AddressRef {
    pub fn path_segment(&self, editor: &mut SyntaxEditor) -> ast::PathSegment {
        let make = SyntaxFactory::new();
        let path_segment = match self {
            ast::AddressRef::NamedAddress(named_address) => {
                make.path_segment_from_text(named_address.name())
            }
            ast::AddressRef::ValueAddress(value_address) => {
                make.path_segment_from_value_address(value_address.address_text())
            }
        };
        editor.add_mappings(make.finish_with_mappings());
        path_segment
    }
}

impl ast::Module {
    pub fn fq_path(&self, editor: &mut SyntaxEditor) -> Option<ast::Path> {
        let make = SyntaxFactory::new();
        let mut segments = vec![];

        let address_ref = self.self_or_parent_address_ref()?;
        segments.push(address_ref.path_segment(editor));

        let module_name = self.name()?.as_string();
        segments.push(make.path_segment_from_text(module_name));
        let path = make.path_from_segments(segments);

        editor.add_mappings(make.finish_with_mappings());

        Some(path)
    }
}

impl ast::NamedElement {
    pub fn use_path(&self, editor: &mut SyntaxEditor) -> Option<ast::Path> {
        let make = SyntaxFactory::new();
        let res = match self {
            ast::NamedElement::Module(module) => {
                let module_path = module.fq_path(editor)?;
                module_path
            }
            _ => {
                let module = self.syntax().containing_module()?;
                let module_path = module.fq_path(editor)?;
                let item_name = self.name()?.as_string();

                let mut segments = vec![];
                segments.extend(module_path.segments());
                segments.push(make.path_segment_from_text(item_name));
                let item_path = make.path_from_segments(segments);

                item_path
            }
        };
        editor.add_mappings(make.finish_with_mappings());
        Some(res)
    }
}
