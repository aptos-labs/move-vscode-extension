use fmt::config::CstFormatConfig;
use fmt::fmt::format_content;
use std::path::Path;

const TESTS_ROOT: &str = "/home/mkurnikov/code/movefmt";

fn check_fmt_file(config: CstFormatConfig, input_path: &str, expected_path: &str) {
    let input = std::fs::read_to_string(Path::new(TESTS_ROOT).join(input_path)).unwrap();
    let expected = std::fs::read_to_string(Path::new(TESTS_ROOT).join(expected_path)).unwrap();
    let actual = format_content(&input, config).unwrap();
    pretty_assertions::assert_eq!(actual, expected, "mismatch for {input_path}");
}

#[test]
fn test_break_line_input1() {
    check_fmt_file(
        CstFormatConfig::default(),
        "tests/break_line/input1.move",
        "tests/break_line/input1.fmt.move",
    );
}

#[test]
fn test_break_line_input2() {
    check_fmt_file(
        CstFormatConfig::default(),
        "tests/break_line/input2.move",
        "tests/break_line/input2.fmt.move",
    );
}

#[test]
fn test_break_line_input3() {
    check_fmt_file(
        CstFormatConfig::default(),
        "tests/break_line/input3.move",
        "tests/break_line/input3.fmt.move",
    );
}

#[test]
#[ignore]
fn test_aptos_framework_account() {
    check_fmt_file(
        CstFormatConfig::default(),
        "tests/aptos_framework_case/account.move",
        "tests/aptos_framework_case/account.fmt.move",
    );
}

#[test]
fn test_aptos_framework_aggregator() {
    check_fmt_file(
        CstFormatConfig::default(),
        "tests/aptos_framework_case/aggregator.move",
        "tests/aptos_framework_case/aggregator.fmt.move",
    );
}

#[test]
#[ignore]
fn test_aptos_framework_aptos_hash() {
    check_fmt_file(
        CstFormatConfig::default(),
        "tests/aptos_framework_case/aptos_hash.move",
        "tests/aptos_framework_case/aptos_hash.fmt.move",
    );
}

#[test]
#[ignore]
fn test_aptos_framework_bit_vector() {
    check_fmt_file(
        CstFormatConfig::default(),
        "tests/aptos_framework_case/bit_vector.move",
        "tests/aptos_framework_case/bit_vector.fmt.move",
    );
}

#[test]
#[ignore]
fn test_aptos_framework_coin() {
    check_fmt_file(
        CstFormatConfig::default(),
        "tests/aptos_framework_case/coin.move",
        "tests/aptos_framework_case/coin.fmt.move",
    );
}

#[test]
#[ignore]
fn test_aptos_framework_coin_spec() {
    check_fmt_file(
        CstFormatConfig::default(),
        "tests/aptos_framework_case/coin.spec.move",
        "tests/aptos_framework_case/coin.spec.fmt.move",
    );
}

#[test]
#[ignore]
fn test_aptos_framework_diem_account() {
    check_fmt_file(
        CstFormatConfig::default(),
        "tests/aptos_framework_case/DiemAccount.move",
        "tests/aptos_framework_case/DiemAccount.fmt.move",
    );
}

#[test]
#[ignore]
fn test_aptos_framework_features_spec() {
    check_fmt_file(
        CstFormatConfig::default(),
        "tests/aptos_framework_case/features.spce.move",
        "tests/aptos_framework_case/features.spce.fmt.move",
    );
}

#[test]
#[ignore]
fn test_aptos_framework_multi_token() {
    check_fmt_file(
        CstFormatConfig::default(),
        "tests/aptos_framework_case/MultiToken.move",
        "tests/aptos_framework_case/MultiToken.fmt.move",
    );
}

#[test]
#[ignore]
fn test_aptos_framework_mutual_inst() {
    check_fmt_file(
        CstFormatConfig::default(),
        "tests/aptos_framework_case/mutual_inst.move",
        "tests/aptos_framework_case/mutual_inst.fmt.move",
    );
}

#[test]
#[ignore]
fn test_aptos_framework_payment_scripts() {
    check_fmt_file(
        CstFormatConfig::default(),
        "tests/aptos_framework_case/PaymentScripts.move",
        "tests/aptos_framework_case/PaymentScripts.fmt.move",
    );
}

#[test]
#[ignore]
fn test_aptos_framework_randomness() {
    check_fmt_file(
        CstFormatConfig::default(),
        "tests/aptos_framework_case/randomness.move",
        "tests/aptos_framework_case/randomness.fmt.move",
    );
}

#[test]
#[ignore]
fn test_aptos_framework_simple() {
    check_fmt_file(
        CstFormatConfig::default(),
        "tests/aptos_framework_case/Simple.move",
        "tests/aptos_framework_case/Simple.fmt.move",
    );
}

#[test]
#[ignore]
fn test_aptos_framework_smart_vector_test() {
    check_fmt_file(
        CstFormatConfig::default(),
        "tests/aptos_framework_case/smart_vector_test.move",
        "tests/aptos_framework_case/smart_vector_test.fmt.move",
    );
}

#[test]
fn test_aptos_framework_table_with_length_spec() {
    check_fmt_file(
        CstFormatConfig::default(),
        "tests/aptos_framework_case/table_with_length.spec.move",
        "tests/aptos_framework_case/table_with_length.spec.fmt.move",
    );
}

#[test]
#[ignore]
fn test_aptos_framework_vector_tests() {
    check_fmt_file(
        CstFormatConfig::default(),
        "tests/aptos_framework_case/vector_tests.move",
        "tests/aptos_framework_case/vector_tests.fmt.move",
    );
}
