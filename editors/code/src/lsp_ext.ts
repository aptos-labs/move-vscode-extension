// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

/**
 * This file mirrors `crates/aptos-analyzer/src/lsp_ext.rs` declarations.
 */

import * as lc from "vscode-languageclient";

export const analyzerStatus = new lc.RequestType<AnalyzerStatusParams, string, void>(
    "aptos-analyzer/analyzerStatus",
);
export type AnalyzerStatusParams = { textDocument?: lc.TextDocumentIdentifier };

export const cancelFlycheck = new lc.NotificationType0("aptos-analyzer/cancelFlycheck");
export const clearFlycheck = new lc.NotificationType0("aptos-analyzer/clearFlycheck");

export const runFlycheck = new lc.NotificationType<{
    textDocument: lc.TextDocumentIdentifier | null
}>("aptos-analyzer/runFlycheck");

export const openServerLogs = new lc.NotificationType0("aptos-analyzer/openServerLogs");

export const serverStatus = new lc.NotificationType<ServerStatusParams>(
    "experimental/serverStatus",
);
export type ServerStatusParams = {
    health: "ok" | "warning" | "error";
    quiescent: boolean;
    message?: string;
};

export const viewSyntaxTree = new lc.RequestType<ViewSyntaxTreeParams, string, void>(
    "aptos-analyzer/viewSyntaxTree",
);
export type ViewSyntaxTreeParams = { textDocument: lc.TextDocumentIdentifier };

export const movefmtVersionError = new lc.NotificationType<MovefmtVersionParams>("aptos-analyzer/movefmtVersionError");

export type MovefmtVersionParams = {
    message: string;
    aptosPath: string | null;
}


