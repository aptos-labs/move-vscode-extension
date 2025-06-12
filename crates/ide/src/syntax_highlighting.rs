mod highlight;
mod highlights;
mod html;
pub mod tags;

use crate::syntax_highlighting::highlights::Highlights;
use ide_db::RootDatabase;
use lang::Semantics;
use syntax::SyntaxKind::WHITESPACE;
use syntax::{AstNode, NodeOrToken, SyntaxNode, TextRange, WalkEvent, ast, match_ast};
use vfs::FileId;

use crate::syntax_highlighting::tags::HlTag;
pub(crate) use html::{highlight_as_html, highlight_as_html_no_style};
use syntax::ast::node_ext::syntax_node::SyntaxTokenExt;
pub(crate) use tags::Highlight;

#[derive(Debug, Clone, Copy)]
pub struct HlRange {
    pub range: TextRange,
    pub highlight: Highlight,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct HighlightConfig {}

pub(crate) fn highlight(
    db: &RootDatabase,
    file_id: FileId,
    range_to_highlight: Option<TextRange>,
) -> Vec<HlRange> {
    let _p = tracing::info_span!("highlight").entered();
    let sema = Semantics::new(db, file_id);

    // Determine the root based on the given range.
    let (root, range_to_highlight) = {
        let file = sema.parse(file_id);
        let source_file = file.syntax();
        match range_to_highlight {
            Some(range) => {
                let covering_node = match source_file.covering_element(range) {
                    NodeOrToken::Node(it) => it,
                    NodeOrToken::Token(it) => it.parent().unwrap_or_else(|| source_file.clone()),
                };
                (covering_node, range)
            }
            None => (source_file.clone(), source_file.text_range()),
        }
    };

    let mut highlights = Highlights::new(root.text_range());
    traverse(&mut highlights, &sema, file_id, &root, range_to_highlight);

    highlights.to_vec()
}

fn traverse(
    highlights: &mut Highlights,
    sema: &Semantics<'_, RootDatabase>,
    _file_id: FileId,
    root: &SyntaxNode,
    range_to_highlight: TextRange,
) {
    // Walk all nodes, keeping track of whether we are inside a macro or not.
    // If in macro, expand it first and highlight the expanded code.
    for walk_event in root.preorder_with_tokens() {
        use WalkEvent::{Enter, Leave};

        let element_range = match &walk_event {
            Enter(it) | Leave(it) => it.text_range(),
        };
        // Element outside of the viewport, no need to highlight
        if range_to_highlight.intersect(element_range).is_none() {
            continue;
        }

        let element = match walk_event {
            Enter(NodeOrToken::Token(token)) if token.is(WHITESPACE) => continue,
            Enter(it) => it,
            Leave(NodeOrToken::Token(_)) => continue,
            Leave(NodeOrToken::Node(_)) => continue,
        };

        let highlight = match element.clone() {
            NodeOrToken::Node(node) => {
                match_ast! {
                    match (node) {
                        ast::NameLike(it) => highlight::name_like::name_like(sema, it),
                        ast::AddressLit(_) => Some(Highlight::new(HlTag::NumericLiteral)),
                        _ => continue,
                    }
                }
            }
            NodeOrToken::Token(t) => highlight::token(t),
        };

        if let Some(highlight) = highlight {
            highlights.add(HlRange {
                range: element_range,
                highlight,
            });
        }
    }
}
