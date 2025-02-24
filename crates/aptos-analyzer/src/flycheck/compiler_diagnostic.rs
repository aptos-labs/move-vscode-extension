use crate::command::ParseFromLine;
use camino::Utf8PathBuf;
use paths::AbsPathBuf;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct CodeRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Deserialize, Debug)]
pub struct DiagnosticLabel {
    pub file_id: String,
    pub style: String,
    pub range: CodeRange,
}

impl DiagnosticLabel {
    pub fn is_primary(&self) -> bool {
        self.style == "Primary"
    }
}

#[derive(Deserialize, Debug)]
pub struct AptosDiagnostic {
    pub severity: String,
    pub code: Option<String>,
    pub message: String,
    pub labels: Vec<DiagnosticLabel>,
}

impl ParseFromLine for AptosDiagnostic {
    fn from_line(line: &str, error: &mut String) -> Option<Self> {
        let mut deserializer = serde_json::Deserializer::from_str(line);

        if let Ok(message) = AptosDiagnostic::deserialize(&mut deserializer) {
            return Some(message);
        }

        error.push_str(line);
        error.push('\n');
        None
    }

    fn from_eof() -> Option<Self> {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::command::ParseFromLine;
    use crate::flycheck::compiler_diagnostic::AptosDiagnostic;

    #[test]
    fn test_basic_compiler_error() {
        let json_line = r#"{"severity":"Error","code":null,"message":"variants not allowed in this context","labels":[{"style":"Primary","file_id":"/home/mkurnikov/code/example-move/sources/modules.move","range":{"start":66,"end":72},"message":""}],"notes":[]}"#;

        let mut error = String::new();
        let check_message = AptosDiagnostic::from_line(json_line, &mut error).unwrap();

        assert_eq!(check_message.message, "variants not allowed in this context");
        assert_eq!(check_message.severity, "Error");

        assert!(check_message.labels.first().unwrap().is_primary());
    }
}
