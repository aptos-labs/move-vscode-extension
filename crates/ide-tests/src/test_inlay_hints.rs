use crate::init_tracing_for_test;
use expect_test::{Expect, expect};
use ide::Analysis;
use ide::inlay_hints::{InlayFieldsToResolve, InlayHint, InlayHintsConfig};
use ide_diagnostics::diagnostic::Diagnostic;
use line_index::LineIndex;
use test_utils::{Marking, apply_markings, remove_markings};

const DISABLED_CONFIG: InlayHintsConfig = InlayHintsConfig {
    // discriminant_hints: DiscriminantHints::Never,
    render_colons: false,
    type_hints: false,
    // parameter_hints: false,
    // sized_bound: false,
    // generic_parameter_hints: GenericParameterHints {
    //     type_hints: false,
    //     lifetime_hints: false,
    //     const_hints: false,
    // },
    // chaining_hints: false,
    // lifetime_elision_hints: LifetimeElisionHints::Never,
    // closure_return_type_hints: ClosureReturnTypeHints::Never,
    // closure_capture_hints: false,
    // adjustment_hints: AdjustmentHints::Never,
    // adjustment_hints_mode: AdjustmentHintsMode::Prefix,
    // adjustment_hints_hide_outside_unsafe: false,
    // binding_mode_hints: false,
    // hide_named_constructor_hints: false,
    // hide_closure_initialization_hints: false,
    hide_closure_parameter_hints: false,
    // closure_style: ClosureStyle::ImplFn,
    // param_names_for_lifetime_elision_hints: false,
    // max_length: None,
    // closing_brace_hints_min_lines: None,
    fields_to_resolve: InlayFieldsToResolve::empty(),
    // implicit_drop_hints: false,
    // range_exclusive_hints: false,
};

const TEST_CONFIG: InlayHintsConfig = InlayHintsConfig {
    type_hints: true,
    // parameter_hints: true,
    // chaining_hints: true,
    // closure_return_type_hints: ClosureReturnTypeHints::WithBlock,
    // binding_mode_hints: true,
    // lifetime_elision_hints: LifetimeElisionHints::Always,
    ..DISABLED_CONFIG
};

#[track_caller]
pub(crate) fn check_inlay_hints(expect: Expect) {
    init_tracing_for_test();

    let source = stdx::trim_indent(expect.data());
    let trimmed_source = remove_markings(&source);

    let (analysis, file_id) = Analysis::from_single_file(trimmed_source.clone());

    let inlay_hints = analysis.inlay_hints(&TEST_CONFIG, file_id, None).unwrap();

    let markings = inlay_hints
        .into_iter()
        .map(|it| {
            let text_range = it.range;
            let message = it.label.to_string();
            Marking { text_range, message }
        })
        .collect();
    let res = apply_markings(trimmed_source.as_str(), markings);
    expect.assert_eq(res.as_str());
}

#[test]
fn test_ident_pat_inlay_hints() {
    // language=Move
    check_inlay_hints(expect![[r#"
module 0x1::m {
    fun main() {
        let a = 1;
          //^ integer
    }
}
    "#]]);
}

#[test]
fn test_item_from_move_stdlib_is_always_local() {
    // language=Move
    check_inlay_hints(expect![[r#"
module std::string {
    struct String { val: u8 }
    public fun get_s(): String {
        String { val: 1 }
    }
}
module 0x1::m {
    use std::string::get_s;
    fun main() {
        let a = get_s();
          //^ String
    }
}
    "#]]);
}

#[test]
fn test_item_from_aptos_stdlib_is_always_local() {
    // language=Move
    check_inlay_hints(expect![[r#"
module aptos_std::string {
    struct String { val: u8 }
    public fun get_s(): String {
        String { val: 1 }
    }
}
module 0x1::m {
    use aptos_std::string::get_s;
    fun main() {
        let a = get_s();
          //^ String
    }
}
    "#]]);
}

#[test]
fn test_ident_pat_in_lambda_param() {
    // language=Move
    check_inlay_hints(expect![[r#"
module 0x1::m {
    fun for_each(v: vector<u8>, f: |u8| u8) {}
    fun main() {
        for_each(vector[], |elem| elem);
                          //^^^^ u8
    }
}
    "#]]);
}
