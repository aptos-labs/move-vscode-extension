// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

/**
 * This file mirrors `crates/aptos-language-server/src/lsp_ext.rs` declarations.
 */

import * as lc from "vscode-languageclient";

export const analyzerStatus = new lc.RequestType<AnalyzerStatusParams, string, void>(
    "aptos-language-server/analyzerStatus",
);
export type AnalyzerStatusParams = { textDocument?: lc.TextDocumentIdentifier };

export const openServerLogs = new lc.NotificationType0("aptos-language-server/openServerLogs");

export const serverStatus = new lc.NotificationType<ServerStatusParams>(
    "experimental/serverStatus",
);
export type ServerStatusParams = {
    health: "ok" | "warning" | "error";
    quiescent: boolean;
    message?: string;
};

export const viewSyntaxTree = new lc.RequestType<ViewSyntaxTreeParams, string, void>(
    "aptos-language-server/viewSyntaxTree",
);
export type ViewSyntaxTreeParams = { textDocument: lc.TextDocumentIdentifier };

export const movefmtVersionError = new lc.NotificationType<MovefmtVersionParams>("aptos-language-server/movefmtVersionError");

export type MovefmtVersionParams = {
    message: string;
    aptosPath: string | null;
}


