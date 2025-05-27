use lsp_types::notification::Notification;
use lsp_types::request::Request;
use lsp_types::{CodeActionKind, Range, TextDocumentIdentifier};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops;

pub enum AnalyzerStatus {}

impl Request for AnalyzerStatus {
    type Params = AnalyzerStatusParams;
    type Result = String;
    const METHOD: &'static str = "aptos-analyzer/analyzerStatus";
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzerStatusParams {
    pub text_document: Option<TextDocumentIdentifier>,
}

pub enum ReloadWorkspace {}

impl Request for ReloadWorkspace {
    type Params = ();
    type Result = ();
    const METHOD: &'static str = "aptos-analyzer/reloadWorkspace";
}

pub enum CancelFlycheck {}

impl Notification for CancelFlycheck {
    type Params = ();
    const METHOD: &'static str = "aptos-analyzer/cancelFlycheck";
}

pub enum RunFlycheck {}

impl Notification for RunFlycheck {
    type Params = RunFlycheckParams;
    const METHOD: &'static str = "aptos-analyzer/runFlycheck";
}

pub enum ClearFlycheck {}

impl Notification for ClearFlycheck {
    type Params = ();
    const METHOD: &'static str = "aptos-analyzer/clearFlycheck";
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RunFlycheckParams {
    pub text_document: Option<TextDocumentIdentifier>,
}

pub enum OpenServerLogs {}

impl Notification for OpenServerLogs {
    type Params = ();
    const METHOD: &'static str = "aptos-analyzer/openServerLogs";
}

pub enum ServerStatusNotification {}

impl Notification for ServerStatusNotification {
    type Params = ServerStatusParams;
    const METHOD: &'static str = "experimental/serverStatus";
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatusParams {
    pub health: Health,
    pub quiescent: bool,
    pub message: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Health {
    Ok,
    Warning,
    Error,
}

impl ops::BitOrAssign for Health {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = match (*self, rhs) {
            (Health::Error, _) | (_, Health::Error) => Health::Error,
            (Health::Warning, _) | (_, Health::Warning) => Health::Warning,
            _ => Health::Ok,
        }
    }
}

pub enum ViewSyntaxTree {}

impl Request for ViewSyntaxTree {
    type Params = ViewSyntaxTreeParams;
    type Result = String;
    const METHOD: &'static str = "aptos-analyzer/viewSyntaxTree";
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ViewSyntaxTreeParams {
    pub text_document: TextDocumentIdentifier,
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeActionData {
    pub code_action_params: lsp_types::CodeActionParams,
    pub id: String,
    pub version: Option<i32>,
}
