// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use camino::Utf8PathBuf;
use lang::item_scope::ItemScope;
use lsp_types::notification::Notification;
use lsp_types::request::Request;
use lsp_types::{CodeActionKind, Position, Range, TextDocumentIdentifier, TextEdit};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops;
use syntax::{SyntaxKind, TextRange};

pub enum AnalyzerStatus {}

impl Request for AnalyzerStatus {
    type Params = AnalyzerStatusParams;
    type Result = String;
    const METHOD: &'static str = "aptos-language-server/analyzerStatus";
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
    const METHOD: &'static str = "aptos-language-server/reloadWorkspace";
}

pub enum OpenServerLogs {}

impl Notification for OpenServerLogs {
    type Params = ();
    const METHOD: &'static str = "aptos-language-server/openServerLogs";
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
    const METHOD: &'static str = "aptos-language-server/viewSyntaxTree";
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

pub enum MovefmtVersionError {}

impl Notification for MovefmtVersionError {
    type Params = MovefmtVersionErrorParams;
    const METHOD: &'static str = "aptos-language-server/movefmtVersionError";
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MovefmtVersionErrorParams {
    pub message: String,
    pub aptos_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InlayHintResolveData {
    pub file_id: u32,
    // This is a string instead of a u64 as javascript can't represent u64 fully
    pub hash: String,
    pub resolve_range: Range,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub version: Option<i32>,
}

/// Information about CodeLens, that is to be resolved.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeLensResolveData {
    pub version: i32,
    pub kind: CodeLensResolveDataKind,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CodeLensResolveDataKind {
    Specs(lsp_types::request::GotoImplementationParams),
    // References(lsp_types::TextDocumentPositionParams),
}

#[derive(Debug, Deserialize, Default)]
pub struct ClientCommandOptions {
    pub commands: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Runnable {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<lsp_types::LocationLink>,
    pub args: AptosRunnableArgs,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AptosRunnableArgs {
    pub workspace_root: Utf8PathBuf,
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub environment: HashMap<String, String>,
}

pub enum OrganizeImports {}

impl Request for OrganizeImports {
    type Params = OrganizeImportsParams;
    type Result = Vec<TextEdit>;
    const METHOD: &'static str = "experimental/organizeImports";
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OrganizeImportsParams {
    pub text_document: TextDocumentIdentifier,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionResolveData {
    pub position: lsp_types::TextDocumentPositionParams,
    pub import: CompletionImport,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub version: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub trigger_character: Option<char>,
    // #[serde(default)]
    // pub for_ref: bool,
    // pub hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionImport {
    pub full_import_path: String,
    pub item_scope: ItemScope,
}
