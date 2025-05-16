#[derive(Debug, Clone)]
pub struct DiagnosticsConfig {
    /// Whether native diagnostics are enabled.
    pub enabled: bool,
    pub unresolved_reference_enabled: bool,
    pub type_checking_enabled: bool,
}

impl DiagnosticsConfig {
    pub fn test_sample() -> Self {
        Self {
            enabled: true,
            unresolved_reference_enabled: true,
            type_checking_enabled: true,
        }
    }
}
