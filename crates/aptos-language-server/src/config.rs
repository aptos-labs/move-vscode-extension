// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

pub(crate) mod config_change;
pub(crate) mod options;
mod utils;
pub mod validation;

use crate::lsp::capabilities::ClientCapabilities;
use camino::Utf8PathBuf;
use ide_completion::config::CompletionConfig;
use ide_db::AllowSnippets;
use paths::AbsPath;
use semver::Version;
use std::collections::HashSet;
use std::fmt;
use std::sync::OnceLock;
use vfs::AbsPathBuf;

use crate::config::options::{DefaultConfigData, FullConfigInput};
use crate::config::utils::find_movefmt_path;
use crate::config::validation::ConfigErrors;
use crate::lsp_ext;
use ide::inlay_hints::{InlayFieldsToResolve, InlayHintsConfig};
use ide_db::assist_config::AssistConfig;
use ide_diagnostics::config::DiagnosticsConfig;
use project_model::DiscoveredManifest;
use serde_derive::{Deserialize, Serialize};
use stdx::itertools::Itertools;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientCommandsConfig {
    pub run_single: bool,
    // pub debug_single: bool,
    pub show_references: bool,
    pub goto_location: bool,
    // pub trigger_parameter_hints: bool,
    // pub rename: bool,
}

/// Configuration for runnable items, such as `main` function or tests.
#[derive(Debug, Clone)]
pub struct RunnablesConfig {
    /// Additional arguments for the `aptos move test`, e.g. `--override-std`.
    pub tests_extra_args: Vec<String>,
    pub prover_extra_args: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LensConfig {
    pub runnables: bool,
    pub specifications: bool,

    // // references
    // pub method_refs: bool,
    // pub refs_adt: bool,   // for Struct, Enum, Union and Trait
    // pub refs_trait: bool, // for Struct, Enum, Union and Trait
    // pub enum_variant_refs: bool,

    // annotations
    pub location: AnnotationLocation,
}

impl LensConfig {
    pub fn any(&self) -> bool {
        self.specifications
    }

    pub fn none(&self) -> bool {
        !self.any()
    }

    pub fn runnable(&self) -> bool {
        self.runnables
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationLocation {
    AboveName,
    AboveWholeItem,
}

impl From<AnnotationLocation> for ide::annotations::AnnotationLocation {
    fn from(location: AnnotationLocation) -> Self {
        match location {
            AnnotationLocation::AboveName => ide::annotations::AnnotationLocation::AboveName,
            AnnotationLocation::AboveWholeItem => ide::annotations::AnnotationLocation::AboveWholeItem,
        }
    }
}

#[derive(Clone, Debug)]
struct ClientInfo {
    name: String,
    version: Option<Version>,
}

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
    client_info: Option<ClientInfo>,

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
            .field("client_info", &self.client_info)
            .field("client_config", &self.client_config)
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
        client_info: Option<lsp_types::ClientInfo>,
    ) -> Self {
        static DEFAULT_CONFIG_DATA: OnceLock<&'static DefaultConfigData> = OnceLock::new();

        Config {
            caps: ClientCapabilities::new(caps),
            discovered_manifests_from_filesystem: Vec::new(),
            root_path,
            client_ws_roots,
            client_info: client_info.map(|it| ClientInfo {
                name: it.name,
                version: it.version.as_deref().map(Version::parse).and_then(Result::ok),
            }),
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

    pub fn assist(&self) -> AssistConfig {
        AssistConfig { allowed: None }
    }

    pub fn completion(&self) -> CompletionConfig {
        CompletionConfig {
            allow_snippets: AllowSnippets::new(self.completion_snippet()),
            enable_imports_on_the_fly: self.completion_autoimport_enable().to_owned()
                && self.caps.has_completion_item_resolve_additionalTextEdits(),
        }
    }

    pub fn diagnostics_config(&self) -> DiagnosticsConfig {
        DiagnosticsConfig {
            enabled: *self.diagnostics_enable(),
            disabled: self.diagnostics_disabled().to_owned(),
            enable_only: self.diagnostics_enableOnly().to_owned(),
            needs_type_annotation: self.diagnostics_needsTypeAnnotation().to_owned(),
            assists_only: false,
        }
    }

    pub fn inlay_hints_config(&self) -> InlayHintsConfig {
        let client_capability_fields = self.inlay_hint_resolve_support_properties();
        InlayHintsConfig {
            render_colons: self.inlayHints_renderColons().to_owned(),
            type_hints: self.inlayHints_typeHints_enable().to_owned(),
            tuple_type_hints: self.inlayHints_typeHints_showForTuples().to_owned(),
            parameter_hints: self.inlayHints_parameterHints_enable().to_owned(),
            range_exclusive_hints: self.inlayHints_rangeExclusiveHints_enable().to_owned(),
            hide_closure_parameter_hints: self.inlayHints_typeHints_hideClosureParameter().to_owned(),
            fields_to_resolve: InlayFieldsToResolve::from_client_capabilities(&client_capability_fields),
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

    pub fn visual_studio_code_version(&self) -> Option<&Version> {
        self.client_info
            .as_ref()
            .filter(|it| it.name.starts_with("Visual Studio Code"))
            .and_then(|it| it.version.as_ref())
    }

    pub fn aptos_path(&self) -> Option<Utf8PathBuf> {
        self.aptosPath().clone()
    }

    pub fn movefmt(&self) -> Option<MovefmtConfig> {
        if let Some(explicit_path) = self.movefmt_path().clone() {
            return Some(MovefmtConfig {
                path: explicit_path,
                extra_args: self.movefmt_extraArgs().clone(),
            });
        }

        let guessed_path = find_movefmt_path()?;
        Some(MovefmtConfig {
            path: guessed_path,
            extra_args: self.movefmt_extraArgs().clone(),
        })
    }

    pub fn discovered_manifests(&self) -> Vec<DiscoveredManifest> {
        // let exclude_dirs = self
        //     .files_excludeDirs()
        //     .iter()
        //     .map(|p| self.root_path.join(p))
        //     .collect::<Vec<_>>();

        let mut manifests = vec![];
        for discovered_manifest in &self.discovered_manifests_from_filesystem {
            // if exclude_dirs
            //     .iter()
            //     .any(|p| discovered_manifest.move_toml_file.starts_with(p))
            // {
            //     continue;
            // }
            manifests.push(discovered_manifest.clone());
        }
        manifests
    }

    pub fn diagnostics_enabled(&self) -> bool {
        self.diagnostics_enable().to_owned()
    }

    pub fn runnables(&self) -> RunnablesConfig {
        RunnablesConfig {
            tests_extra_args: self.tests_extraArgs().clone(),
            prover_extra_args: self.prover_extraArgs().clone(),
        }
    }

    pub fn lens(&self) -> LensConfig {
        LensConfig {
            runnables: *self.lens_enable() && *self.lens_run_enable(),
            specifications: *self.lens_enable() && *self.lens_specifications_enable(),
            location: *self.lens_location(),
        }
    }

    pub fn commands(&self) -> Option<lsp_ext::ClientCommandOptions> {
        self.experimental("commands")
    }

    pub fn client_commands(&self) -> ClientCommandsConfig {
        let commands = self.commands().map(|it| it.commands).unwrap_or_default();

        let get = |name: &str| commands.iter().any(|it| it == name);

        ClientCommandsConfig {
            run_single: get("move-on-aptos.runSingle"),
            show_references: get("move-on-aptos.showReferences"),
            goto_location: get("move-on-aptos.gotoLocation"),
        }
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

    fn experimental<T: serde::de::DeserializeOwned>(&self, index: &'static str) -> Option<T> {
        serde_json::from_value(self.0.experimental.as_ref()?.get(index)?.clone()).ok()
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
    use stdext::line_endings::LineEndings;
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
            Ok(old_contents) => {
                let (old_contents, _) = LineEndings::normalize(old_contents);
                let (contents, _) = LineEndings::normalize(contents.to_string());
                if old_contents == contents {
                    return Ok(());
                }
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

    #[cfg(not(windows))]
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
