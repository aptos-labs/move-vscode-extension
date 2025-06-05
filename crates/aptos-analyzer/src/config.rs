pub(crate) mod config_change;
pub(crate) mod options;
pub mod validation;

use crate::lsp::capabilities::ClientCapabilities;
use camino::Utf8PathBuf;
use ide_completion::config::CompletionConfig;
use ide_db::AllowSnippets;
use paths::AbsPath;
use std::collections::HashSet;
use std::fmt;
use std::sync::OnceLock;
use vfs::AbsPathBuf;

use crate::config::options::{DefaultConfigData, FullConfigInput};
use crate::config::validation::ConfigErrors;
use crate::flycheck::{AptosCliOptions, FlycheckConfig};
use ide::inlay_hints::{InlayFieldsToResolve, InlayHintsConfig};
use ide_db::assist_config::AssistConfig;
use ide_diagnostics::config::DiagnosticsConfig;
use project_model::DiscoveredManifest;
use serde_derive::{Deserialize, Serialize};
use stdx::itertools::Itertools;

#[derive(Clone)]
pub struct Config {
    /// Projects that have a Move.toml in a
    /// parent directory, so we can discover them by walking the
    /// file system.
    discovered_manifests_from_filesystem: Vec<DiscoveredManifest>,

    /// The workspace roots as registered by the LSP client
    client_ws_roots: Vec<AbsPathBuf>,
    caps: ClientCapabilities,
    root_path: AbsPathBuf,

    default_config: &'static DefaultConfigData,

    /// Config node that obtains its initial value during the server initialization and
    /// by receiving a `lsp_types::notification::DidChangeConfiguration`.
    client_config: (FullConfigInput, ConfigErrors),

    /// Use case : It is an error to have an empty value for `check_command`.
    /// Since it is a `global` command at the moment, its final value can only be determined by
    /// traversing through `global` configs and the `client` config. However the non-null value constraint
    /// is config level agnostic, so this requires an independent error storage
    validation_errors: ConfigErrors,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field(
                "discovered_manifests_from_filesystem",
                &self.discovered_manifests_from_filesystem,
            )
            .field("client_ws_roots", &self.client_ws_roots)
            .field("caps", &self.caps)
            .field("root_path", &self.root_path)
            // .field("snippets", &self.snippets)
            // .field("client_info", &self.client_info)
            .field("client_config", &self.client_config)
            // .field("user_config", &self.user_config)
            // .field("source_root_parent_map", &self.source_root_parent_map)
            .field("validation_errors", &self.validation_errors)
            .finish()
    }
}

// Delegate capability fetching methods
impl std::ops::Deref for Config {
    type Target = ClientCapabilities;

    fn deref(&self) -> &Self::Target {
        &self.caps
    }
}

impl Config {
    pub fn new(
        root_path: AbsPathBuf,
        caps: lsp_types::ClientCapabilities,
        client_ws_roots: Vec<AbsPathBuf>,
    ) -> Self {
        static DEFAULT_CONFIG_DATA: OnceLock<&'static DefaultConfigData> = OnceLock::new();

        Config {
            caps: ClientCapabilities::new(caps),
            discovered_manifests_from_filesystem: Vec::new(),
            root_path,
            client_ws_roots,
            client_config: (FullConfigInput::default(), ConfigErrors(vec![])),
            default_config: DEFAULT_CONFIG_DATA.get_or_init(|| Box::leak(Box::default())),
            validation_errors: Default::default(),
        }
    }

    pub fn rediscover_packages(&mut self) {
        let discovered_manifests = DiscoveredManifest::discover_all(&self.client_ws_roots);
        if discovered_manifests.is_empty() {
            tracing::error!("failed to find any manifests in {:?}", &self.client_ws_roots);
        }
        self.discovered_manifests_from_filesystem = discovered_manifests;
    }

    pub fn add_client_ws_root(&mut self, paths: impl Iterator<Item = AbsPathBuf>) {
        self.client_ws_roots.extend(paths);
    }

    pub fn remove_client_ws_root(&mut self, path: &AbsPath) {
        if let Some(position) = self.client_ws_roots.iter().position(|it| it == path) {
            self.client_ws_roots.remove(position);
        }
    }

    pub fn files(&self) -> FilesConfig {
        FilesConfig {
            watcher: match self.files_watcher() {
                FilesWatcherDef::Client if self.did_change_watched_files_dynamic_registration() => {
                    FilesWatcher::Client
                }
                _ => FilesWatcher::Server,
            },
            // exclude: self.files_excludeDirs().iter().map(|it| self.root_path.join(it)).collect(),
        }
    }

    pub fn autorefresh_on_move_toml_changes(&self) -> bool {
        self.aptos_autoreload().to_owned()
    }

    pub fn assist(&self) -> AssistConfig {
        AssistConfig { allowed: None }
    }

    pub fn completion(&self) -> CompletionConfig {
        // let client_capability_fields = self.completion_resolve_support_properties();
        CompletionConfig {
            // enable_postfix_completions: self.completion_postfix_enable(source_root).to_owned(),
            // enable_imports_on_the_fly: self.completion_autoimport_enable(source_root).to_owned()
            //     && self.caps.completion_item_edit_resolve(),
            // enable_self_on_the_fly: self.completion_autoself_enable(source_root).to_owned(),
            // enable_private_editable: self.completion_privateEditable_enable(source_root).to_owned(),
            // full_function_signatures: self
            //     .completion_fullFunctionSignatures_enable(source_root)
            //     .to_owned(),
            // callable: match self.completion_callable_snippets(source_root) {
            //     CallableCompletionDef::FillArguments => Some(CallableSnippets::FillArguments),
            //     CallableCompletionDef::AddParentheses => Some(CallableSnippets::AddParentheses),
            //     CallableCompletionDef::None => None,
            // },
            // add_semicolon_to_unit: *self.completion_addSemicolonToUnit(source_root),
            allow_snippets: AllowSnippets::new(self.completion_snippet()),
            // insert_use: self.insert_use_config(source_root),
            // prefer_no_std: self.imports_preferNoStd(source_root).to_owned(),
            // prefer_prelude: self.imports_preferPrelude(source_root).to_owned(),
            // prefer_absolute: self.imports_prefixExternPrelude(source_root).to_owned(),
            // snippets: self.snippets.clone().to_vec(),
            // limit: self.completion_limit(source_root).to_owned(),
            // enable_term_search: self.completion_termSearch_enable(source_root).to_owned(),
            // term_search_fuel: self.completion_termSearch_fuel(source_root).to_owned() as u64,
            // fields_to_resolve: if self.client_is_neovim() {
            //     CompletionFieldsToResolve::empty()
            // } else {
            //     CompletionFieldsToResolve::from_client_capabilities(&client_capability_fields)
            // },
            // exclude_flyimport: self
            //     .completion_autoimport_exclude(source_root)
            //     .iter()
            //     .map(|it| match it {
            //         AutoImportExclusion::Path(path) => {
            //             (path.clone(), ide_completion::AutoImportExclusionType::Always)
            //         }
            //         AutoImportExclusion::Verbose { path, r#type } => (
            //             path.clone(),
            //             match r#type {
            //                 AutoImportExclusionType::Always => {
            //                     ide_completion::AutoImportExclusionType::Always
            //                 }
            //                 AutoImportExclusionType::Methods => {
            //                     ide_completion::AutoImportExclusionType::Methods
            //                 }
            //             },
            //         ),
            //     })
            //     .collect(),
            // exclude_traits: self.completion_excludeTraits(source_root),
        }
    }

    pub fn diagnostics_config(&self) -> DiagnosticsConfig {
        DiagnosticsConfig {
            enabled: *self.diagnostics_enable(),
            unresolved_reference_enabled: *self.diagnostics_enableUnresolvedReference(),
            type_checking_enabled: *self.diagnostics_enableTypeChecking(),
            assists_only: false,
        }
    }

    pub fn inlay_hints(&self) -> InlayHintsConfig {
        let client_capability_fields = self.inlay_hint_resolve_support_properties();

        InlayHintsConfig {
            render_colons: self.inlayHints_renderColons().to_owned(),
            type_hints: self.inlayHints_typeHints_enable().to_owned(),
            // sized_bound: self.inlayHints_implicitSizedBoundHints_enable().to_owned(),
            // parameter_hints: self.inlayHints_parameterHints_enable().to_owned(),
            // generic_parameter_hints: GenericParameterHints {
            //     type_hints: self.inlayHints_genericParameterHints_type_enable().to_owned(),
            //     lifetime_hints: self.inlayHints_genericParameterHints_lifetime_enable().to_owned(),
            //     const_hints: self.inlayHints_genericParameterHints_const_enable().to_owned(),
            // },
            // chaining_hints: self.inlayHints_chainingHints_enable().to_owned(),
            // discriminant_hints: match self.inlayHints_discriminantHints_enable() {
            //     DiscriminantHintsDef::Always => ide::DiscriminantHints::Always,
            //     DiscriminantHintsDef::Never => ide::DiscriminantHints::Never,
            //     DiscriminantHintsDef::Fieldless => ide::DiscriminantHints::Fieldless,
            // },
            // closure_return_type_hints: match self.inlayHints_closureReturnTypeHints_enable() {
            //     ClosureReturnTypeHintsDef::Always => ide::ClosureReturnTypeHints::Always,
            //     ClosureReturnTypeHintsDef::Never => ide::ClosureReturnTypeHints::Never,
            //     ClosureReturnTypeHintsDef::WithBlock => ide::ClosureReturnTypeHints::WithBlock,
            // },
            // lifetime_elision_hints: match self.inlayHints_lifetimeElisionHints_enable() {
            //     LifetimeElisionDef::Always => ide::LifetimeElisionHints::Always,
            //     LifetimeElisionDef::Never => ide::LifetimeElisionHints::Never,
            //     LifetimeElisionDef::SkipTrivial => ide::LifetimeElisionHints::SkipTrivial,
            // },
            // hide_named_constructor_hints: self
            //     .inlayHints_typeHints_hideNamedConstructor()
            //     .to_owned(),
            // hide_closure_initialization_hints: self
            //     .inlayHints_typeHints_hideClosureInitialization()
            //     .to_owned(),
            hide_closure_parameter_hints: self.inlayHints_typeHints_hideClosureParameter().to_owned(),
            // closure_style: match self.inlayHints_closureStyle() {
            //     ClosureStyle::ImplFn => hir::ClosureStyle::ImplFn,
            //     ClosureStyle::RustAnalyzer => hir::ClosureStyle::RANotation,
            //     ClosureStyle::WithId => hir::ClosureStyle::ClosureWithId,
            //     ClosureStyle::Hide => hir::ClosureStyle::Hide,
            // },
            // closure_capture_hints: self.inlayHints_closureCaptureHints_enable().to_owned(),
            // adjustment_hints: match self.inlayHints_expressionAdjustmentHints_enable() {
            //     AdjustmentHintsDef::Always => ide::AdjustmentHints::Always,
            //     AdjustmentHintsDef::Never => match self.inlayHints_reborrowHints_enable() {
            //         ReborrowHintsDef::Always | ReborrowHintsDef::Mutable => {
            //             ide::AdjustmentHints::ReborrowOnly
            //         }
            //         ReborrowHintsDef::Never => ide::AdjustmentHints::Never,
            //     },
            //     AdjustmentHintsDef::Reborrow => ide::AdjustmentHints::ReborrowOnly,
            // },
            // adjustment_hints_mode: match self.inlayHints_expressionAdjustmentHints_mode() {
            //     AdjustmentHintsModeDef::Prefix => ide::AdjustmentHintsMode::Prefix,
            //     AdjustmentHintsModeDef::Postfix => ide::AdjustmentHintsMode::Postfix,
            //     AdjustmentHintsModeDef::PreferPrefix => ide::AdjustmentHintsMode::PreferPrefix,
            //     AdjustmentHintsModeDef::PreferPostfix => ide::AdjustmentHintsMode::PreferPostfix,
            // },
            // adjustment_hints_hide_outside_unsafe: self
            //     .inlayHints_expressionAdjustmentHints_hideOutsideUnsafe()
            //     .to_owned(),
            // binding_mode_hints: self.inlayHints_bindingModeHints_enable().to_owned(),
            // param_names_for_lifetime_elision_hints: self
            //     .inlayHints_lifetimeElisionHints_useParameterNames()
            //     .to_owned(),
            // max_length: self.inlayHints_maxLength().to_owned(),
            // closing_brace_hints_min_lines: if self.inlayHints_closingBraceHints_enable().to_owned()
            // {
            //     Some(self.inlayHints_closingBraceHints_minLines().to_owned())
            // } else {
            //     None
            // },
            fields_to_resolve: InlayFieldsToResolve::from_client_capabilities(&client_capability_fields),
            // implicit_drop_hints: self.inlayHints_implicitDrops_enable().to_owned(),
            // range_exclusive_hints: self.inlayHints_rangeExclusiveHints_enable().to_owned(),
        }
    }

    pub fn inlay_hint_resolve_support_properties(&self) -> HashSet<&str> {
        self.0
            .text_document
            .as_ref()
            .and_then(|text| text.inlay_hint.as_ref())
            .and_then(|inlay_hint_caps| inlay_hint_caps.resolve_support.as_ref())
            .map(|inlay_resolve| inlay_resolve.properties.iter())
            .into_iter()
            .flatten()
            .map(|s| s.as_str())
            .collect()
    }

    // // VSCode is our reference implementation, so we allow ourselves to work around issues by
    // // special casing certain versions
    // pub fn visual_studio_code_version(&self) -> Option<&Version> {
    //     self.client_info
    //         .as_ref()
    //         .filter(|it| it.name.starts_with("Visual Studio Code"))
    //         .and_then(|it| it.version.as_ref())
    // }

    pub(crate) fn flycheck_config(&self) -> Option<FlycheckConfig> {
        let cli_path = self.aptos_cli_path()?;
        let options = AptosCliOptions {
            extra_args: self.extra_args().clone(),
            ..AptosCliOptions::default()
        };
        let command = self.check_command();
        Some(FlycheckConfig::new(
            self.check_on_save(),
            cli_path,
            command,
            options,
        ))
    }

    pub fn check_on_save(&self) -> bool {
        *self.checkOnSave()
    }

    pub fn extra_args(&self) -> &Vec<String> {
        self.check_extraArgs()
    }

    pub fn aptos_cli_path(&self) -> Option<Utf8PathBuf> {
        self.aptosPath().clone()
    }

    pub fn movefmt(&self) -> Option<MovefmtConfig> {
        let path = self.movefmt_path().clone()?;
        Some(MovefmtConfig {
            path,
            extra_args: self.movefmt_extraArgs().clone(),
        })
    }

    pub fn discovered_manifests(&self) -> Vec<DiscoveredManifest> {
        let exclude_dirs = self
            .files_excludeDirs()
            .iter()
            .map(|p| self.root_path.join(p))
            .collect::<Vec<_>>();

        let mut manifests = vec![];
        for discovered_manifest in &self.discovered_manifests_from_filesystem {
            if exclude_dirs
                .iter()
                .any(|p| discovered_manifest.move_toml_file.starts_with(p))
            {
                continue;
            }
            manifests.push(discovered_manifest.clone());
        }
        manifests
    }

    pub fn diagnostics_enabled(&self) -> bool {
        self.diagnostics_enable().to_owned()
    }

    pub fn snippet_text_edit(&self) -> bool {
        self.experimental_bool("snippetTextEdit")
    }

    pub fn snippet_cap(&self) -> Option<AllowSnippets> {
        // FIXME: Also detect the proposed lsp version at caps.workspace.workspaceEdit.snippetEditSupport
        // once lsp-types has it.
        AllowSnippets::new(self.snippet_text_edit())
    }

    pub fn main_loop_num_threads(&self) -> usize {
        num_cpus::get_physical()
    }

    pub fn json_schema() -> serde_json::Value {
        let mut s = FullConfigInput::json_schema();

        fn sort_objects_by_field(json: &mut serde_json::Value) {
            if let serde_json::Value::Object(object) = json {
                let old = std::mem::take(object);
                old.into_iter()
                    .sorted_by(|(k, _), (k2, _)| k.cmp(k2))
                    .for_each(|(k, mut v)| {
                        sort_objects_by_field(&mut v);
                        object.insert(k, v);
                    });
            }
        }
        sort_objects_by_field(&mut s);
        s
    }

    pub fn root_path(&self) -> &AbsPathBuf {
        &self.root_path
    }

    pub fn is_under_ws_roots(&self, path: &AbsPath) -> bool {
        self.client_ws_roots
            .iter()
            .any(|ws_root| path.starts_with(ws_root))
    }

    pub fn caps(&self) -> &ClientCapabilities {
        &self.caps
    }
}

#[derive(Debug, Clone)]
pub struct MovefmtConfig {
    pub path: Utf8PathBuf,
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FilesConfig {
    pub watcher: FilesWatcher,
}

#[derive(Debug, Clone)]
pub enum FilesWatcher {
    Client,
    Server,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FilesWatcherDef {
    Client,
    Notify,
    Server,
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;
    use std::fs;
    use std::path::{Path, PathBuf};
    use stdx::is_ci;

    /// Returns the path to the root directory of `rust-analyzer` project.
    /// todo: into test-utils
    pub fn project_root() -> Utf8PathBuf {
        let dir = env!("CARGO_MANIFEST_DIR");
        Utf8PathBuf::from_path_buf(PathBuf::from(dir).parent().unwrap().parent().unwrap().to_owned())
            .unwrap()
    }

    /// Checks that the `file` has the specified `contents`. If that is not the
    /// case, updates the file and then fails the test.
    #[track_caller]
    pub fn ensure_file_contents(file: &Path, contents: &str) {
        if let Err(()) = try_ensure_file_contents(file, contents) {
            panic!("Some files were not up-to-date");
        }
    }

    /// Checks that the `file` has the specified `contents`. If that is not the
    /// case, updates the file and return an Error.
    pub fn try_ensure_file_contents(file: &Path, contents: &str) -> Result<(), ()> {
        match fs::read_to_string(file) {
            Ok(old_contents) if normalize_newlines(&old_contents) == normalize_newlines(contents) => {
                return Ok(());
            }
            _ => (),
        }
        let display_path = file.strip_prefix(project_root()).unwrap_or(file);
        eprintln!(
            "\n\x1b[31;1merror\x1b[0m: {} was not up-to-date, updating\n",
            display_path.display()
        );
        if is_ci() {
            eprintln!("    NOTE: run `cargo test` locally and commit the updated files\n");
        }
        if let Some(parent) = file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(file, contents).unwrap();
        Err(())
    }

    fn normalize_newlines(s: &str) -> String {
        s.replace("\r\n", "\n")
    }

    #[test]
    fn generate_package_json_config() {
        let s = Config::json_schema();

        let schema = format!("{s:#}");
        let mut schema = schema
            .trim_start_matches('[')
            .trim_end_matches(']')
            .replace("  ", "    ")
            .replace('\n', "\n        ")
            .trim_start_matches('\n')
            .trim_end()
            .to_owned();
        schema.push_str(",\n");

        // Transform the asciidoc form link to markdown style.
        //
        // https://link[text] => [text](https://link)
        let url_matches = schema.match_indices("https://");
        let mut url_offsets = url_matches.map(|(idx, _)| idx).collect::<Vec<usize>>();
        url_offsets.reverse();
        for idx in url_offsets {
            let link = &schema[idx..];
            // matching on whitespace to ignore normal links
            if let Some(link_end) = link.find([' ', '[']) {
                if link.chars().nth(link_end) == Some('[') {
                    if let Some(link_text_end) = link.find(']') {
                        let link_text = link[link_end..(link_text_end + 1)].to_string();

                        schema.replace_range((idx + link_end)..(idx + link_text_end + 1), "");
                        schema.insert(idx, '(');
                        schema.insert(idx + link_end + 1, ')');
                        schema.insert_str(idx, &link_text);
                    }
                }
            }
        }

        let package_json_path = project_root().join("editors/code/package.json");
        let mut package_json = fs::read_to_string(&package_json_path).unwrap();

        let start_marker =
            "            {\n                \"title\": \"$generated-start\"\n            },\n";
        let end_marker = "            {\n                \"title\": \"$generated-end\"\n            }\n";

        let start = package_json.find(start_marker).unwrap() + start_marker.len();
        let end = package_json.find(end_marker).unwrap();

        let p = remove_ws(&package_json[start..end]);
        let s = remove_ws(&schema);
        if !p.contains(&s) {
            package_json.replace_range(start..end, &schema);
            ensure_file_contents(package_json_path.as_std_path(), &package_json)
        }
    }

    // #[test]
    // fn generate_config_documentation() {
    //     let docs_path = project_root().join("docs/user/generated_config.adoc");
    //     let expected = FullConfigInput::manual();
    //     ensure_file_contents(docs_path.as_std_path(), &expected);
    // }

    fn remove_ws(text: &str) -> String {
        text.replace(char::is_whitespace, "")
    }
}
