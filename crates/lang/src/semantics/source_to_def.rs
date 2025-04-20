//! Maps *syntax* of various definitions to their semantic ids.
//!
//! This is a very interesting module, and, in some sense, can be considered the
//! heart of the IDE parts of rust-analyzer.
//!
//! This module solves the following problem:
//!
//!     Given a piece of syntax, find the corresponding semantic definition (def).
//!
//! This problem is a part of more-or-less every IDE feature implemented. Every
//! IDE functionality (like goto to definition), conceptually starts with a
//! specific cursor position in a file. Starting with this text offset, we first
//! figure out what syntactic construct are we at: is this a pattern, an
//! expression, an item definition.
//!
//! Knowing only the syntax gives us relatively little info. For example,
//! looking at the syntax of the function we can realize that it is a part of an
//! `impl` block, but we won't be able to tell what trait function the current
//! function overrides, and whether it does that correctly. For that, we need to
//! go from [`ast::Fn`] to [`crate::Function`], and that's exactly what this
//! module does.
//!
//! As syntax trees are values and don't know their place of origin/identity,
//! this module also requires [`InFile`] wrappers to understand which specific
//! real or macro-expanded file the tree comes from.
//!
//! The actual algorithm to resolve syntax to def is curious in two aspects:
//!
//! * It is recursive
//! * It uses the inverse algorithm (what is the syntax for this def?)
//!
//! Specifically, the algorithm goes like this:
//!
//! 1. Find the syntactic container for the syntax. For example, field's
//!    container is the struct, and structs container is a module.
//! 2. Recursively get the def corresponding to container.
//! 3. Ask the container def for all child defs. These child defs contain
//!    the answer and answer's siblings.
//! 4. For each child def, ask for it's source.
//! 5. The child def whose source is the syntax node we've started with
//!    is the answer.
//!
//! It's interesting that both Roslyn and Kotlin contain very similar code
//! shape.
//!
//! Let's take a look at Roslyn:
//!
//!   <https://github.com/dotnet/roslyn/blob/36a0c338d6621cc5fe34b79d414074a95a6a489c/src/Compilers/CSharp/Portable/Compilation/SyntaxTreeSemanticModel.cs#L1403-L1429>
//!   <https://sourceroslyn.io/#Microsoft.CodeAnalysis.CSharp/Compilation/SyntaxTreeSemanticModel.cs,1403>
//!
//! The `GetDeclaredType` takes `Syntax` as input, and returns `Symbol` as
//! output. First, it retrieves a `Symbol` for parent `Syntax`:
//!
//! * <https://sourceroslyn.io/#Microsoft.CodeAnalysis.CSharp/Compilation/SyntaxTreeSemanticModel.cs,1423>
//!
//! Then, it iterates parent symbol's children, looking for one which has the
//! same text span as the original node:
//!
//!   <https://sourceroslyn.io/#Microsoft.CodeAnalysis.CSharp/Compilation/SyntaxTreeSemanticModel.cs,1786>
//!
//! Now, let's look at Kotlin:
//!
//!   <https://github.com/JetBrains/kotlin/blob/a288b8b00e4754a1872b164999c6d3f3b8c8994a/idea/idea-frontend-fir/idea-fir-low-level-api/src/org/jetbrains/kotlin/idea/fir/low/level/api/FirModuleResolveStateImpl.kt#L93-L125>
//!
//! This function starts with a syntax node (`KtExpression` is syntax, like all
//! `Kt` nodes), and returns a def. It uses
//! `getNonLocalContainingOrThisDeclaration` to get syntactic container for a
//! current node. Then, `findSourceNonLocalFirDeclaration` gets `Fir` for this
//! parent. Finally, `findElementIn` function traverses `Fir` children to find
//! one with the same source we originally started with.
//!
//! One question is left though -- where does the recursion stops? This happens
//! when we get to the file syntax node, which doesn't have a syntactic parent.
//! In that case, we loop through all the crates that might contain this file
//! and look for a module whose source is the given file.
//!
//! Note that the logic in this module is somewhat fundamentally imprecise --
//! due to conditional compilation and `#[path]` attributes, there's no
//! injective mapping from syntax nodes to defs. This is not an edge case --
//! more or less every item in a `lib.rs` is a part of two distinct crates: a
//! library with `--cfg test` and a library without.
//!
//! At the moment, we don't really handle this well and return the first answer
//! that works. Ideally, we should first let the caller to pick a specific
//! active crate for a given position, and then provide an API to resolve all
//! syntax nodes against this specific crate.

use std::collections::HashMap;
use syntax::SyntaxNode;
use vfs::FileId;

#[derive(Default)]
pub(super) struct SourceToDefCache {
    pub(super) root_to_file_cache: HashMap<SyntaxNode, FileId>,
}

impl SourceToDefCache {
    pub(super) fn cache(
        root_to_file_cache: &mut HashMap<SyntaxNode, FileId>,
        root_node: SyntaxNode,
        file_id: FileId,
    ) {
        assert!(root_node.parent().is_none());
        let prev = root_to_file_cache.insert(root_node, file_id);
        assert!(prev.is_none() || prev == Some(file_id));
    }
}
