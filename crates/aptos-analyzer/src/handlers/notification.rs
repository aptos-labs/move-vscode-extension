use crate::config::config_change::ConfigChange;
use crate::global_state::{FetchPackagesRequest, GlobalState};
use crate::lsp::from_proto;
use crate::lsp::utils::apply_document_changes;
use crate::lsp_ext::RunFlycheckParams;
use crate::mem_docs::DocumentData;
use crate::{Config, reload};
use camino::Utf8PathBuf;
use lsp_types::{
    CancelParams, DidChangeConfigurationParams, DidChangeTextDocumentParams,
    DidChangeWatchedFilesParams, DidChangeWorkspaceFoldersParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DidSaveTextDocumentParams, WorkDoneProgressCancelParams,
};
use paths::AbsPathBuf;
use std::ops::Not;
use std::sync::Arc;
use stdx::itertools::Itertools;
use vfs::VfsPath;
use vfs::loader::Handle;

#[tracing::instrument(level = "info", skip_all)]
pub(crate) fn handle_cancel(state: &mut GlobalState, params: CancelParams) -> anyhow::Result<()> {
    let id: lsp_server::RequestId = match params.id {
        lsp_types::NumberOrString::Number(id) => id.into(),
        lsp_types::NumberOrString::String(id) => id.into(),
    };
    state.cancel(id);
    Ok(())
}

#[tracing::instrument(level = "info", skip_all)]
pub(crate) fn handle_work_done_progress_cancel(
    state: &mut GlobalState,
    params: WorkDoneProgressCancelParams,
) -> anyhow::Result<()> {
    if let lsp_types::NumberOrString::String(s) = &params.token {
        if let Some(id) = s.strip_prefix("aptos-analyzer/flycheck/") {
            if let Ok(id) = id.parse::<u32>() {
                if let Some(flycheck) = state.flycheck.get(id as usize) {
                    flycheck.cancel();
                }
            }
        }
    }

    // Just ignore this. It is OK to continue sending progress
    // notifications for this token, as the client can't know when
    // we accepted notification.
    Ok(())
}

#[tracing::instrument(level = "info", skip_all)]
pub(crate) fn handle_did_open_text_document(
    state: &mut GlobalState,
    params: DidOpenTextDocumentParams,
) -> anyhow::Result<()> {
    if let Ok(path) = from_proto::vfs_path(&params.text_document.uri) {
        let already_exists = state
            .mem_docs
            .insert(
                path.clone(),
                DocumentData::new(
                    params.text_document.version,
                    params.text_document.text.clone().into_bytes(),
                ),
            )
            .is_err();
        if already_exists {
            tracing::error!("duplicate DidOpenTextDocument: {}", path);
        }

        state
            .vfs
            .write()
            .0
            .set_file_contents(path, Some(params.text_document.text.into_bytes()));
    }
    Ok(())
}

#[tracing::instrument(level = "info", skip_all)]
pub(crate) fn handle_did_change_text_document(
    state: &mut GlobalState,
    params: DidChangeTextDocumentParams,
) -> anyhow::Result<()> {
    if let Ok(path) = from_proto::vfs_path(&params.text_document.uri) {
        let Some(DocumentData { version, data }) = state.mem_docs.get_mut(&path) else {
            tracing::error!(?path, "unexpected DidChangeTextDocument");
            return Ok(());
        };
        // The version passed in DidChangeTextDocument is the version after all edits are applied
        // so we should apply it before the vfs is notified.
        *version = params.text_document.version;

        let new_contents = apply_document_changes(
            state.config.negotiated_encoding(),
            std::str::from_utf8(data).unwrap(),
            params.content_changes,
        )
        .into_bytes();
        if *data != new_contents {
            data.clone_from(&new_contents);
            state.vfs.write().0.set_file_contents(path, Some(new_contents));
        }
    }
    Ok(())
}

#[tracing::instrument(level = "info", skip_all)]
pub(crate) fn handle_did_close_text_document(
    state: &mut GlobalState,
    params: DidCloseTextDocumentParams,
) -> anyhow::Result<()> {
    if let Ok(path) = from_proto::vfs_path(&params.text_document.uri) {
        if state.mem_docs.remove(&path).is_err() {
            tracing::error!("orphan DidCloseTextDocument: {}", path);
        }

        if let Some((file_id, _)) = state.vfs.read().0.file_id(&path) {
            state.diagnostics.clear_native_for(file_id);
        }

        // state.semantic_tokens_cache.lock().remove(&params.text_document.uri);

        if let Some(path) = path.as_path() {
            state.vfs_loader.handle.invalidate(path.to_path_buf());
        }
    }
    Ok(())
}

#[tracing::instrument(level = "info", skip_all)]
pub(crate) fn handle_did_save_text_document(
    state: &mut GlobalState,
    params: DidSaveTextDocumentParams,
) -> anyhow::Result<()> {
    if let Ok(vfs_path) = from_proto::vfs_path(&params.text_document.uri) {
        // Re-fetch workspaces if a workspace related file has changed
        if let Some(path) = vfs_path.as_path() {
            // FIXME: We should move this check into a QueuedTask and do semantic resolution of
            // the files. There is only so much we can tell syntactically from the path.
            if reload::should_refresh_for_file_change(
                path, /*, ChangeKind::Modify, additional_files*/
            ) {
                state.fetch_packages_from_fs_queue.request_op(
                    format!("workspace vfs file change saved {path}"),
                    FetchPackagesRequest { force_reload_deps: false },
                );
            }
        }
        if !state.config.check_on_save() || run_flycheck(state, vfs_path) {
            return Ok(());
        }
    } else if state.config.check_on_save() {
        // No specific flycheck was triggered, so let's trigger all of them.
        for flycheck in state.flycheck.iter() {
            flycheck.restart_workspace();
        }
    }

    Ok(())
}

#[tracing::instrument(level = "info", skip_all)]
pub(crate) fn handle_did_change_configuration(
    state: &mut GlobalState,
    _params: DidChangeConfigurationParams,
) -> anyhow::Result<()> {
    // As stated in https://github.com/microsoft/language-server-protocol/issues/676,
    // this notification's parameters should be ignored and the actual config queried separately.
    state.send_request::<lsp_types::request::WorkspaceConfiguration>(
        lsp_types::ConfigurationParams {
            items: vec![lsp_types::ConfigurationItem {
                scope_uri: None,
                section: Some("aptos-analyzer".to_owned()),
            }],
        },
        |this, resp| {
            tracing::debug!("config update response: '{:?}", resp);
            let lsp_server::Response { error, result, .. } = resp;

            match (error, result) {
                (Some(err), _) => {
                    tracing::error!("failed to fetch the server settings: {:?}", err)
                }
                (None, Some(mut configs)) => {
                    if let Some(json) = configs.get_mut(0) {
                        let config = Config::clone(&*this.config);
                        let mut change = ConfigChange::default();
                        change.change_client_config(json.take());

                        let (config, errors, _) = config.apply_change(change);
                        this.config_errors = errors.is_empty().not().then_some(errors);

                        // Client config changes neccesitates .update_config method to be called.
                        this.update_configuration(config);
                    }
                }
                (None, None) => {
                    tracing::error!("received empty server settings response from the client")
                }
            }
        },
    );

    Ok(())
}

#[tracing::instrument(level = "info", skip_all)]
pub(crate) fn handle_did_change_workspace_folders(
    state: &mut GlobalState,
    params: DidChangeWorkspaceFoldersParams,
) -> anyhow::Result<()> {
    let config = Arc::make_mut(&mut state.config);

    for workspace_folder in params.event.removed {
        let Ok(path) = workspace_folder.uri.to_file_path() else {
            continue;
        };
        let Ok(path) = Utf8PathBuf::from_path_buf(path) else {
            continue;
        };
        let Ok(path) = AbsPathBuf::try_from(path) else {
            continue;
        };
        config.remove_client_ws_root(&path);
    }

    let added_ws_root = params
        .event
        .added
        .into_iter()
        .filter_map(|it| it.uri.to_file_path().ok())
        .filter_map(|it| Utf8PathBuf::from_path_buf(it).ok())
        .filter_map(|it| AbsPathBuf::try_from(it).ok());
    config.add_client_ws_root(added_ws_root);

    config.rediscover_packages();

    let req = FetchPackagesRequest { force_reload_deps: false };
    state
        .fetch_packages_from_fs_queue
        .request_op("client workspaces changed".to_owned(), req);

    Ok(())
}

#[tracing::instrument(level = "info", skip_all)]
pub(crate) fn handle_did_change_watched_files(
    state: &mut GlobalState,
    params: DidChangeWatchedFilesParams,
) -> anyhow::Result<()> {
    for change in params.changes.iter().unique_by(|&it| &it.uri) {
        if let Ok(path) = from_proto::abs_path(&change.uri) {
            state.vfs_loader.handle.invalidate(path);
        }
    }
    Ok(())
}

#[tracing::instrument(level = "info", skip_all)]
fn run_flycheck(state: &mut GlobalState, vfs_path: VfsPath) -> bool {
    let file_id = state.vfs.read().0.file_id(&vfs_path);
    if let Some((saved_file_id, _)) = file_id {
        let world = state.snapshot();

        let mut updated = false;
        let task = move || -> Result<(), ide::Cancelled> {
            let saved_file_path = world
                .file_id_to_file_path(saved_file_id)
                .as_path()
                .expect("cannot be none, as it's been filtered at the first if-let")
                .to_owned();

            let workspace_ids = world
                .all_packages
                .iter()
                .enumerate()
                .filter(|(_, ws)| ws.contains_file(saved_file_path.as_path()));

            // Find and trigger corresponding flychecks
            'flychecks: for flycheck in world.flycheck.iter() {
                for (ws_id, _) in workspace_ids.clone() {
                    if ws_id == flycheck.ws_id() {
                        updated = true;
                        flycheck.restart_workspace();
                        continue 'flychecks;
                    }
                }
            }
            // No specific flycheck was triggered, so let's trigger all of them.
            if !updated {
                for flycheck in world.flycheck.iter() {
                    flycheck.restart_workspace();
                }
            }
            Ok(())
        };
        state
            .task_pool
            .handle
            .spawn_with_sender(stdx::thread::ThreadIntent::Worker, move |_| {
                if let Err(e) = std::panic::catch_unwind(task) {
                    tracing::error!("flycheck task panicked: {e:?}")
                }
            });
        true
    } else {
        false
    }
}

#[tracing::instrument(level = "info", skip_all)]
pub(crate) fn handle_cancel_flycheck(state: &mut GlobalState, _: ()) -> anyhow::Result<()> {
    state.flycheck.iter().for_each(|flycheck| flycheck.cancel());
    Ok(())
}

#[tracing::instrument(level = "info", skip_all)]
pub(crate) fn handle_clear_flycheck(state: &mut GlobalState, _: ()) -> anyhow::Result<()> {
    state.diagnostics.clear_check_all();
    Ok(())
}

#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn handle_run_flycheck(
    state: &mut GlobalState,
    params: RunFlycheckParams,
) -> anyhow::Result<()> {
    if let Some(text_document) = params.text_document {
        if let Ok(vfs_path) = from_proto::vfs_path(&text_document.uri) {
            if run_flycheck(state, vfs_path) {
                return Ok(());
            }
        }
    }
    // No specific flycheck was triggered, so let's trigger all of them.
    for ws_flycheck in state.flycheck.iter() {
        ws_flycheck.restart_workspace();
    }
    Ok(())
}
