#[derive(Debug, Clone)]
pub struct DiagnosticsConfig {
    /// Whether native diagnostics are enabled.
    pub enabled: bool,
    // pub proc_macros_enabled: bool,
    // pub proc_attr_macros_enabled: bool,
    // pub disable_experimental: bool,
    // pub disabled: FxHashSet<String>,
    // pub expr_fill_default: ExprFillDefaultMode,
    // pub style_lints: bool,
    // FIXME: We may want to include a whole `AssistConfig` here
    // pub snippet_cap: Option<SnippetCap>,
    // pub insert_use: InsertUseConfig,
    // pub prefer_no_std: bool,
    // pub prefer_prelude: bool,
    // pub prefer_absolute: bool,
    // pub term_search_fuel: u64,
    // pub term_search_borrowck: bool,
}

impl DiagnosticsConfig {
    pub fn test_sample() -> Self {
        Self {
            enabled: true,
            // disable_experimental: Default::default(),
            // disabled: Default::default(),
            // expr_fill_default: Default::default(),
            // style_lints: true,
            // snippet_cap: SnippetCap::new(true),
            // insert_use: InsertUseConfig {
            //     granularity: ImportGranularity::Preserve,
            //     enforce_granularity: false,
            //     prefix_kind: PrefixKind::Plain,
            //     group: false,
            //     skip_glob_imports: false,
            // },
            // prefer_no_std: false,
            // prefer_prelude: true,
            // prefer_absolute: false,
            // term_search_fuel: 400,
            // term_search_borrowck: true,
        }
    }
}
