use crate::config::options::FullConfigInput;
use crate::config::{ConfigErrorInner, ConfigErrors};
use crate::Config;
use serde::de::DeserializeOwned;
use std::iter;
use triomphe::Arc;

impl Config {
    /// Changes made to client and global configurations will partially not be reflected even after `.apply_change()` was called.
    /// The return tuple's bool component signals whether the `GlobalState` should call its `update_configuration()` method.
    fn apply_change_with_sink(&self, change: ConfigChange) -> (Config, bool) {
        let mut config = self.clone();
        // todo: flycheck
        // config.validation_errors = ConfigErrors::default();

        let mut should_update = false;

        if let Some(json) = change.client_config_change {
            tracing::info!("updating config from JSON: {:#}", json);

            if !(json.is_null() || json.as_object().is_some_and(|it| it.is_empty())) {
                // note: can be copied and uncommented to support config migrations
                // patch_old_style::patch_json_for_outdated_configs(&mut json);

                let mut json_errors = vec![];
                config.client_config = (
                    FullConfigInput::from_json(json, &mut json_errors),
                    ConfigErrors(
                        json_errors
                            .into_iter()
                            .map(|(a, b)| ConfigErrorInner::Json {
                                config_key: a,
                                error: b,
                            })
                            .map(Arc::new)
                            .collect(),
                    ),
                );
            }
            should_update = true;
        }

        // todo: flycheck
        // if config.check_command(None).is_empty() {
        //     config.validation_errors.0.push(Arc::new(ConfigErrorInner::Json {
        //         config_key: "/check/command".to_owned(),
        //         error: serde_json::Error::custom("expected a non-empty string"),
        //     }));
        // }

        (config, should_update)
    }

    /// Given `change` this generates a new `Config`, thereby collecting errors of type `ConfigError`.
    /// If there are changes that have global/client level effect, the last component of the return type
    /// will be set to `true`, which should be used by the `GlobalState` to update itself.
    pub fn apply_change(&self, change: ConfigChange) -> (Config, ConfigErrors, bool) {
        let (config, should_update) = self.apply_change_with_sink(change);
        let e = ConfigErrors(
            config
                .client_config
                .1
                 .0
                .iter()
                // todo: flycheck
                // .chain(config.validation_errors.0.iter())
                .cloned()
                .collect(),
        );
        (config, e, should_update)
    }
}

#[derive(Default, Debug)]
pub struct ConfigChange {
    client_config_change: Option<serde_json::Value>,
}

impl ConfigChange {
    pub fn change_client_config(&mut self, change: serde_json::Value) {
        self.client_config_change = Some(change);
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
