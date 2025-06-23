use expect_test::{Expect, expect};
use ide_db::symbol_index::Query;
use test_utils::{SourceMark, apply_source_marks, fixtures};

fn check_symbols(source: &str, query: Query, with_symbols: Expect) {
    let (analysis, _) = fixtures::from_single_file(source.to_string());
    let symbols = analysis.symbol_search(query, 10).unwrap();

    let marks = symbols
        .iter()
        .map(|it| SourceMark::at_range(it.focus_or_full_range(), ""))
        .collect::<Vec<_>>();
    with_symbols.assert_eq(&apply_source_marks(source, marks));
}

#[test]
fn test_world_symbols() {
    // language=Move
    let source = r#"
module 0x1::main {
    struct SMain { val: u8 }
    enum Main { One, Two }
    fun main() {
    }
}
    "#;
    check_symbols(
        source,
        Query::new("main".to_string()),
        // language=Move
        expect![[r#"
            module 0x1::main {
                struct SMain { val: u8 }
                     //^^^^^
                enum Main { One, Two }
                   //^^^^
                fun main() {
                  //^^^^
                }
            }
    "#]],
    );
    check_symbols(
        source,
        Query::new("one".to_string()),
        // language=Move
        expect![[r#"
            module 0x1::main {
                struct SMain { val: u8 }
                enum Main { One, Two }
                          //^^^
                fun main() {
                }
            }
    "#]],
    );
    check_symbols(
        source,
        Query::new("two".to_string()),
        // language=Move
        expect![[r#"
            module 0x1::main {
                struct SMain { val: u8 }
                enum Main { One, Two }
                               //^^^
                fun main() {
                }
            }
    "#]],
    );
}

#[test]
fn test_world_symbols_case_sensitive() {
    // language=Move
    let source = r#"
module 0x1::main {
    struct SMain { val: u8 }
    enum Main { One, Two }
    fun main() {
    }
}
    "#;
    check_symbols(
        source,
        Query::new("main".to_string()).case_sensitive(),
        // language=Move
        expect![[r#"
            module 0x1::main {
                struct SMain { val: u8 }
                enum Main { One, Two }
                fun main() {
                  //^^^^
                }
            }
    "#]],
    );
}
