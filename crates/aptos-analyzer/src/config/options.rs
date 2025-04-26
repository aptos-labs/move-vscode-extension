mod macros;

use crate::config::FilesWatcherDef;
use camino::Utf8PathBuf;
use macros::{config_data, default_str, default_val, impl_for_config_data};
use serde::de::DeserializeOwned;
use std::iter;
use stdx::format_to_acc;
use stdx::itertools::Itertools;

// Defines the server-side configuration of the rust-analyzer. We generate
// *parts* of VS Code's `package.json` config from this. Run `cargo test` to
// re-generate that file.
//
// However, editor specific config, which the server doesn't know about, should
// be specified directly in `package.json`.
//
// To deprecate an option by replacing it with another name use `new_name` | `old_name` so that we keep
// parsing the old name.
config_data! {
    /// Global configuration options
    global: struct GlobalDefaultConfigData <- GlobalConfigInput -> {
        /// Path to the `aptos-cli` executable.
        aptosPath: Option<Utf8PathBuf>                         = None,

        /// Automatically refresh project info on `Move.toml` changes.
        aptos_autoreload: bool           = true,

        /// Run the check command for diagnostics on save.
        checkOnSave: bool                         = true,

        /// Extra arguments for `aptos move compile`.
        check_extraArgs: Vec<String>             = vec![],

        /// Whether to show native aptos-analyzer diagnostics.
        diagnostics_enable: bool                = true,

        /// Whether to show "Unresolved reference" diagnostic.
        diagnostics_enableUnresolvedReference: bool                = true,
    }
}

config_data! {
    /// Configs that only make sense when they are set by a client. As such they can only be defined
    /// by setting them using client's settings (e.g `settings.json` on VS Code).
    client: struct ClientDefaultConfigData <- ClientConfigInput -> {

        /// Controls file watching implementation.
        files_watcher: FilesWatcherDef = FilesWatcherDef::Client,
    }
}

#[derive(Default, Debug, Clone)]
pub(crate) struct DefaultConfigData {
    global: GlobalDefaultConfigData,
    // workspace: WorkspaceDefaultConfigData,
    // local: LocalDefaultConfigData,
    client: ClientDefaultConfigData,
}

/// All of the config levels, all fields `Option<T>`, to describe fields that are actually set by
/// some rust-analyzer.toml file or JSON blob. An empty rust-analyzer.toml corresponds to
/// all fields being None.
#[derive(Debug, Clone, Default)]
pub(crate) struct FullConfigInput {
    global: GlobalConfigInput,
    // workspace: WorkspaceConfigInput,
    // local: LocalConfigInput,
    client: ClientConfigInput,
}

impl FullConfigInput {
    pub fn from_json(
        mut json: serde_json::Value,
        error_sink: &mut Vec<(String, serde_json::Error)>,
    ) -> FullConfigInput {
        FullConfigInput {
            global: GlobalConfigInput::from_json(&mut json, error_sink),
            // local: LocalConfigInput::from_json(&mut json, error_sink),
            client: ClientConfigInput::from_json(&mut json, error_sink),
            // workspace: WorkspaceConfigInput::from_json(&mut json, error_sink),
        }
    }

    pub(crate) fn json_schema() -> serde_json::Value {
        schema(&Self::schema_fields())
    }

    #[cfg(test)]
    pub(crate) fn manual() -> String {
        manual(&Self::schema_fields())
    }

    fn schema_fields() -> Vec<SchemaField> {
        let mut fields = Vec::new();
        GlobalConfigInput::schema_fields(&mut fields);
        // LocalConfigInput::schema_fields(&mut fields);
        // ClientConfigInput::schema_fields(&mut fields);
        // WorkspaceConfigInput::schema_fields(&mut fields);
        fields.sort_by_key(|&(x, ..)| x);
        fields
            .iter()
            .tuple_windows()
            .for_each(|(a, b)| assert!(a.0 != b.0, "{a:?} duplicate field"));
        fields
    }
}

fn get_field_json<T: DeserializeOwned>(
    json: &mut serde_json::Value,
    error_sink: &mut Vec<(String, serde_json::Error)>,
    field: &'static str,
    alias: Option<&'static str>,
) -> Option<T> {
    // XXX: check alias first, to work around the VS Code where it pre-fills the
    // defaults instead of sending an empty object.
    alias
        .into_iter()
        .chain(iter::once(field))
        .filter_map(move |field| {
            let mut pointer = field.replace('_', "/");
            pointer.insert(0, '/');
            json.pointer_mut(&pointer)
                .map(|it| serde_json::from_value(it.take()).map_err(|e| (e, pointer)))
        })
        .find(Result::is_ok)
        .and_then(|res| match res {
            Ok(it) => Some(it),
            Err((e, pointer)) => {
                tracing::warn!("Failed to deserialize config field at {}: {:?}", pointer, e);
                error_sink.push((pointer, e));
                None
            }
        })
}

type SchemaField = (&'static str, &'static str, &'static [&'static str], String);

fn schema(fields: &[SchemaField]) -> serde_json::Value {
    let map = fields
        .iter()
        .map(|(field, ty, doc, default)| {
            let name = field.replace('_', ".");
            let category = name
                .find('.')
                .map(|end| String::from(&name[..end]))
                .unwrap_or("general".into());
            let name = format!("aptos-analyzer.{name}");
            let props = field_props(field, ty, doc, default);
            serde_json::json!({
                "title": category,
                "properties": {
                    name: props
                }
            })
        })
        .collect::<Vec<_>>();
    map.into()
}

fn field_props(field: &str, ty: &str, doc: &[&str], default: &str) -> serde_json::Value {
    let doc = doc_comment_to_string(doc);
    let doc = doc.trim_end_matches('\n');
    assert!(
        doc.ends_with('.') && doc.starts_with(char::is_uppercase),
        "bad docs for {field}: {doc:?}"
    );
    let default = default.parse::<serde_json::Value>().unwrap();

    let mut map = serde_json::Map::default();
    macro_rules! set {
        ($($key:literal: $value:tt),*$(,)?) => {{$(
            map.insert($key.into(), serde_json::json!($value));
        )*}};
    }
    set!("markdownDescription": doc);
    set!("default": default);

    match ty {
        "bool" => set!("type": "boolean"),
        "usize" => set!("type": "integer", "minimum": 0),
        "String" => set!("type": "string"),
        "Vec<String>" => set! {
            "type": "array",
            "items": { "type": "string" },
        },
        "Vec<Utf8PathBuf>" => set! {
            "type": "array",
            "items": { "type": "string" },
        },
        "FxHashSet<String>" => set! {
            "type": "array",
            "items": { "type": "string" },
            "uniqueItems": true,
        },
        "FxHashMap<Box<str>, Box<[Box<str>]>>" => set! {
            "type": "object",
        },
        "FxHashMap<String, SnippetDef>" => set! {
            "type": "object",
        },
        "FxHashMap<String, String>" => set! {
            "type": "object",
        },
        "FxHashMap<Box<str>, u16>" => set! {
            "type": "object",
        },
        "FxHashMap<String, Option<String>>" => set! {
            "type": "object",
        },
        "Option<usize>" => set! {
            "type": ["null", "integer"],
            "minimum": 0,
        },
        "Option<u16>" => set! {
            "type": ["null", "integer"],
            "minimum": 0,
            "maximum": 65535,
        },
        "Option<String>" => set! {
            "type": ["null", "string"],
        },
        "Option<Utf8PathBuf>" => set! {
            "type": ["null", "string"],
        },
        "Option<bool>" => set! {
            "type": ["null", "boolean"],
        },
        "Option<Vec<String>>" => set! {
            "type": ["null", "array"],
            "items": { "type": "string" },
        },
        "FilesWatcherDef" => set! {
            "type": "string",
            "enum": ["client", "server"],
            "enumDescriptions": [
                "Use the client (editor) to watch files for changes",
                "Use server-side file watching",
            ],
        },
        _ => panic!("missing entry for {ty}: {default} (field {field})"),
    }

    map.into()
}

#[cfg(test)]
fn manual(fields: &[SchemaField]) -> String {
    fields
        .iter()
        .fold(String::new(), |mut acc, (field, _ty, doc, default)| {
            let name = format!("aptos-analyzer.{}", field.replace('_', "."));
            let doc = doc_comment_to_string(doc);
            if default.contains('\n') {
                format_to_acc!(
                    acc,
                    r#"[[{name}]]{name}::
+
--
Default:
----
{default}
----
{doc}
--
"#
                )
            } else {
                format_to_acc!(acc, "[[{name}]]{name} (default: `{default}`)::\n+\n--\n{doc}--\n")
            }
        })
}

fn doc_comment_to_string(doc: &[&str]) -> String {
    doc.iter()
        .map(|it| it.strip_prefix(' ').unwrap_or(it))
        .fold(String::new(), |mut acc, it| format_to_acc!(acc, "{it}\n"))
}
