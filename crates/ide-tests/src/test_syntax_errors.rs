use ide_diagnostics::config::DiagnosticsConfig;
use ide_diagnostics::diagnostic::Diagnostic;
use syntax::{SyntaxError, TextRange};
use test_utils::fixtures;

#[test]
fn test_right_brace_expected() {
    // language=Move
    let source = r#"
module 0x1::m {
    "#;
    let (analysis, file_id) = fixtures::from_single_file(source.to_string());

    let diagnostic_config = DiagnosticsConfig::test_sample();
    let diagnostics = analysis.syntax_diagnostics(&diagnostic_config, file_id).unwrap();

    let expected = Diagnostic::new_syntax_error(
        file_id,
        &SyntaxError::new("expected R_CURLY".to_string(), TextRange::at(16.into(), 0.into())),
    );
    // assert_eq!(diagnostics, vec![expected]);
}
