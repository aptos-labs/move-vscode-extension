use crate::RangeInfo;
use crate::hover::HoverResult;
use base_db::SourceDatabase;
use lang::node_ext::item_spec::ItemSpecExt;
use syntax::SyntaxKind::*;
use syntax::ast::node_ext::syntax_element::SyntaxElementExt;
use syntax::files::InFileExt;
use syntax::{SyntaxToken, ast};
use vfs::FileId;

pub(crate) fn spec_keyword_docs(
    db: &dyn SourceDatabase,
    file_id: FileId,
    kw_token: SyntaxToken,
) -> Option<RangeInfo<HoverResult>> {
    let item_spec = kw_token
        .ancestor_strict::<ast::ItemSpec>()
        .map(|it| it.in_file(file_id));
    let item_kind = item_spec.and_then(|it| it.item(db)).map(|it| it.kind());
    let doc_string = match kw_token.kind() {
        ASSERT_KW => {
            // language=Markdown
            r#"
An `assert` statement inside a spec block indicates a condition that must hold when control reaches that block.
If the condition does not hold, an error is reported by the Move Prover.
        "#
        }
        ASSUME_KW => {
            // language=Markdown
            r#"
An `assume` statement blocks executions violating the condition in the statement.
        "#
        }
        REQUIRES_KW => {
            // language=Markdown
            r#"
The `requires` condition is a spec block member that postulates a pre-condition for a function.
The Move Prover will produce verification errors for functions that are called with violating pre-conditions.

A `requires` is different from an `aborts_if`: in the latter case, the function can be called,
and any aborts it produces will be propagated to the caller context. In the `requires` case,
the Move Prover will not allow the function to be called in the first place.
Nevertheless, the function can still be called at runtime if verification is skipped.
Because of this, `requires` are rare in Move specifications, and `aborts_if` are more common.
Specifically, `requires` should be avoided for public APIs.
        "#
        }
        ENSURES_KW => {
            // language=Markdown
            r#"
The `ensures` condition postulates a post-condition for a function that must be satisfied when the function
terminates successfully (i.e., does not abort).
The Move Prover will verify each ensures to this end.
        "#
        }
        MODIFIES_KW => {
            // language=Markdown
            r#"
The `modifies` condition is used to provide permissions to a function to modify global storage.
The annotation itself comprises a list of global access expressions.
It is specifically used together with `pragma opaque;` function specifications.
```
fun mutate_at(addr: address) acquires S {
    let s = borrow_global_mut<S>(addr);
    s.x = 2;
}
spec mutate_at {
    pragma opaque;
    modifies global<S>(addr);
}
```
        "#
        }
        ABORTS_IF_KW => {
            // language=Markdown
            r#"
The `aborts_if` condition is a spec block member that can appear only in a function context.
It specifies conditions under which the function aborts.

In the following example, we specify that the function increment aborts if the Counter resource does not
exist at address a (recall that a is the name of the parameter of increment).
```
module 0x42::m {
  spec increment {
    aborts_if !exists<Counter>(a);
  }
}
```

The `aborts_if` condition can be augmented with code:
```
module 0x42::m {
  fun get_value(addr: address): u64 {
    if exists<Counter>(addr) {
        abort 3
    };
    // ...
  }
  spec get_value {
    aborts_if !exists<Counter>(addr) with 3;
  }
}
```
        "#
        }
        ABORTS_WITH_KW => {
            // language=Markdown
            r#"
The `aborts_with` condition allows specifying with which codes a function can abort,
independent under which condition. It is similar to a ‘throws’ clause in languages like Java.
```
module 0x42::m {
  fun get_one_off(addr: address): u64 {
    if exists<Counter>(addr) {
        abort 3
    };
    borrow_global<Counter>(addr).value - 1
  }
  spec get_one_off {
    aborts_with 3, EXECUTION_FAILURE;
  }
}
```
If the function aborts with any other or none of the specified codes, a verification error will be produced.

The `aborts_with` condition can be combined with `aborts_if` conditions.
In this case, the `aborts_with` specifies any other codes with which the function may abort,
in addition to the ones given in the `aborts_if`:
```
aborts_if !exists<Counter>(addr) with 3;
aborts_with EXECUTION_FAILURE;
```
        "#
        }
        // language=Markdown
        INVARIANT_KW if item_kind.is_some_and(|it| it == FUN) => {
            r#"
The `invariant` condition on a function is simply a shortcut for a `requires` and `ensures` with the same predicate.
Thus, the following:
```
invariant global<Counter>(a).value < 128;
```
is equivalent to:
```
requires global<Counter>(a).value < 128;
ensures global<Counter>(a).value < 128;
```
        "#
        }
        // language=Markdown
        INVARIANT_KW if item_kind.is_some_and(|it| it == STRUCT || it == ENUM) => {
            r#"
When the `invariant` condition is applied to a struct, it expresses a well-formedness property of the struct data.
Any instance of this struct that is currently not mutated will satisfy this property (with exceptions as outlined below).

For example, we can postulate an invariant on our counter that it never must exceed the value of 127:
```
spec Counter {
    invariant value < 128;
}
```
A struct invariant is checked by the Move Prover whenever the struct value is constructed (packed).
While the struct is mutated (e.g., via a `&mut Counter`) the invariant does not hold (but see exception below).
In general, we consider mutation as an implicit unpack, and end of mutation as a pack.
        "#
        }
        _ => {
            return None;
        }
    };

    Some(RangeInfo::new(
        kw_token.text_range(),
        HoverResult {
            doc_string: doc_string.to_string(),
        },
    ))
}
