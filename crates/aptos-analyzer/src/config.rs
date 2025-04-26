pub(crate) mod config_change;
pub(crate) mod options;

use crate::lsp::capabilities::ClientCapabilities;
use camino::Utf8PathBuf;
use ide_completion::config::CompletionConfig;
use ide_db::SnippetCap;
use paths::AbsPath;
use std::fmt;
use std::sync::OnceLock;
use vfs::AbsPathBuf;

use crate::config::options::{DefaultConfigData, FullConfigInput};
use crate::flycheck::{AptosOptions, FlycheckConfig};
use base_db::package_root::PackageRootId;
use ide_db::assist_config::AssistConfig;
use ide_diagnostics::config::DiagnosticsConfig;
use project_model::manifest_path::ManifestPath;
use serde_derive::{Deserialize, Serialize};
use std::sync::Arc;
use stdx::itertools::Itertools;

#[derive(Clone)]
pub struct Config {
    /// Projects that have a Move.toml in a
    /// parent directory, so we can discover them by walking the
    /// file system.
    discovered_manifests_from_filesystem: Vec<ManifestPath>,

    /// The workspace roots as registered by the LSP client
    workspace_roots: Vec<AbsPathBuf>,
    caps: ClientCapabilities,
    root_path: AbsPathBuf,

    default_config: &'static DefaultConfigData,

    /// Config node that obtains its initial value during the server initialization and
    /// by receiving a `lsp_types::notification::DidChangeConfiguration`.
    client_config: (FullConfigInput, ConfigErrors),
    // todo: flycheck
    // /// Use case : It is an error to have an empty value for `check_command`.
    // /// Since it is a `global` command at the moment, its final value can only be determined by
    // /// traversing through `global` configs and the `client` config. However the non-null value constraint
    // /// is config level agnostic, so this requires an independent error storage
    // validation_errors: ConfigErrors,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field(
                "discovered_projects_from_filesystem",
                &self.discovered_manifests_from_filesystem,
            )
            // .field("discovered_projects_from_command", &self.discovered_projects_from_command)
            .field("workspace_roots", &self.workspace_roots)
            .field("caps", &self.caps)
            .field("root_path", &self.root_path)
            // .field("snippets", &self.snippets)
            // .field("client_info", &self.client_info)
            .field("client_config", &self.client_config)
            // .field("user_config", &self.user_config)
            // .field("source_root_parent_map", &self.source_root_parent_map)
            // .field("validation_errors", &self.validation_errors)
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

#[derive(Debug)]
pub enum ConfigErrorInner {
    Json { config_key: String, error: serde_json::Error },
    ParseError { reason: String },
}

#[derive(Clone, Debug, Default)]
pub struct ConfigErrors(Vec<Arc<ConfigErrorInner>>);

impl ConfigErrors {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for ConfigErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let errors = self.0.iter().format_with("\n", |inner, f| match &**inner {
            ConfigErrorInner::Json { config_key: key, error: e } => {
                f(key)?;
                f(&": ")?;
                f(e)
            }
            ConfigErrorInner::ParseError { reason } => f(reason),
        });
        write!(
            f,
            "invalid config value{}:\n{}",
            if self.0.len() == 1 { "" } else { "s" },
            errors
        )
    }
}

impl std::error::Error for ConfigErrors {}

impl Config {
    pub fn new(
        root_path: AbsPathBuf,
        caps: lsp_types::ClientCapabilities,
        workspace_roots: Vec<AbsPathBuf>,
    ) -> Self {
        static DEFAULT_CONFIG_DATA: OnceLock<&'static DefaultConfigData> = OnceLock::new();

        Config {
            caps: ClientCapabilities::new(caps),
            discovered_manifests_from_filesystem: Vec::new(),
            root_path,
            workspace_roots,
            client_config: (FullConfigInput::default(), ConfigErrors(vec![])),
            default_config: DEFAULT_CONFIG_DATA.get_or_init(|| Box::leak(Box::default())),
        }
    }

    pub fn rediscover_workspaces(&mut self) {
        let discovered_manifests = ManifestPath::discover_all(&self.workspace_roots);
        tracing::info!("discovered manifests: {:?}", discovered_manifests);
        if discovered_manifests.is_empty() {
            tracing::error!("failed to find any manifests in {:?}", &self.workspace_roots);
        }
        self.discovered_manifests_from_filesystem = discovered_manifests;
    }

    pub fn remove_workspace(&mut self, path: &AbsPath) {
        if let Some(position) = self.workspace_roots.iter().position(|it| it == path) {
            self.workspace_roots.remove(position);
        }
    }

    pub fn add_workspaces(&mut self, paths: impl Iterator<Item = AbsPathBuf>) {
        self.workspace_roots.extend(paths);
    }

    pub fn files(&self) -> FilesConfig {
        FilesConfig {
            watcher: match self.files_watcher() {
                FilesWatcherDef::Client if self.did_change_watched_files_dynamic_registration() => {
                    FilesWatcher::Client
                }
                _ => FilesWatcher::Server,
            },
            exclude: vec![],
            // exclude: self.files_excludeDirs().iter().map(|it| self.root_path.join(it)).collect(),
        }
    }

    pub fn cargo_autoreload_config(&self) -> bool {
        self.aptos_autoreload().to_owned()
    }

    pub fn assist(&self /*source_root: Option<SourceRootId>*/) -> AssistConfig {
        AssistConfig {
            snippet_cap: self.snippet_cap(),
            allowed: None,
            // insert_use: self.insert_use_config(source_root),
            code_action_grouping: self.code_action_group(),
        }
    }

    pub fn completion(&self /*source_root: Option<SourceRootId>*/) -> CompletionConfig {
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
            snippet_cap: SnippetCap::new(self.completion_snippet()),
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

    pub fn diagnostics(&self) -> DiagnosticsConfig {
        DiagnosticsConfig {
            enabled: *self.diagnostics_enable(),
            unresolved_reference_enabled: *self.diagnostics_enable_unresolvedReference(),
            // disable_experimental: !self.diagnostics_experimental_enable(source_root),
            // disabled: self.diagnostics_disabled.clone(),
            // expr_fill_default: match self.assist_expressionFillDefault(source_root) {
            //     ExprFillDefaultDef::Todo => ExprFillDefaultMode::Todo,
            //     ExprFillDefaultDef::Default => ExprFillDefaultMode::Default,
            // },
            // snippet_cap: self.snippet_cap(),
            // insert_use: self.insert_use_config(source_root),
            // style_lints: self.diagnostics_styleLints_enable(source_root).to_owned(),
        }
    }

    pub(crate) fn flycheck_config(&self) -> Option<FlycheckConfig> {
        let cli_path = self.aptos_cli_path()?;
        let options = AptosOptions {
            extra_args: self.extra_args().clone(),
            ..AptosOptions::default()
        };
        Some(FlycheckConfig::new(cli_path, "compile".to_string(), options))
    }

    pub fn check_on_save(&self) -> bool {
        *self.checkOnSave()
    }

    pub fn extra_args(&self /*source_root: Option<PackageRootId>*/) -> &Vec<String> {
        self.check_extraArgs(/*source_root*/)
    }

    pub fn aptos_cli_path(&self) -> Option<Utf8PathBuf> {
        self.aptosPath().clone()
    }

    pub fn discovered_manifests(&self) -> Vec<DiscoveredManifest> {
        // let exclude_dirs: Vec<_> =
        //     self.files_excludeDirs().iter().map(|p| self.root_path.join(p)).collect();
        // let exclude_dirs = vec![];

        let mut manifests = vec![];
        for manifest_from_fs in &self.discovered_manifests_from_filesystem {
            // if exclude_dirs.iter().any(|p| manifest_path.starts_with(p)) {
            //     continue;
            // }
            let buf: Utf8PathBuf = manifest_from_fs.to_path_buf().into();
            manifests.push(buf);
        }

        manifests
            .iter()
            .filter_map(|manifest_buf| {
                let path = self.root_path.join(manifest_buf);
                ManifestPath::from_manifest_file(path)
                    .map_err(|e| tracing::error!("failed to load linked project: {}", e))
                    .ok()
                    .map(|manifest| DiscoveredManifest { path: manifest })
            })
            .collect()
    }

    pub fn publish_diagnostics(&self) -> bool {
        self.diagnostics_enable().to_owned()
    }

    pub fn snippet_text_edit(&self) -> bool {
        self.experimental_bool("snippetTextEdit")
    }

    pub fn snippet_cap(&self) -> Option<SnippetCap> {
        // FIXME: Also detect the proposed lsp version at caps.workspace.workspaceEdit.snippetEditSupport
        // once lsp-types has it.
        SnippetCap::new(self.snippet_text_edit())
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

    pub fn caps(&self) -> &ClientCapabilities {
        &self.caps
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DiscoveredManifest {
    pub path: ManifestPath,
}

impl From<ManifestPath> for DiscoveredManifest {
    fn from(v: ManifestPath) -> Self {
        DiscoveredManifest { path: v }
    }
}

#[derive(Debug, Clone)]
pub struct FilesConfig {
    pub watcher: FilesWatcher,
    pub exclude: Vec<AbsPathBuf>,
}

#[derive(Debug, Clone)]
pub enum FilesWatcher {
    Client,
    Server,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
enum FilesWatcherDef {
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
