/**
 * This file mirrors `crates/aptos-analyzer/src/lsp_ext.rs` declarations.
 */

import * as lc from "vscode-languageclient";

export const cancelFlycheck = new lc.NotificationType0("aptos-analyzer/cancelFlycheck");
export const clearFlycheck = new lc.NotificationType0("aptos-analyzer/clearFlycheck");

export const runFlycheck = new lc.NotificationType<{
    textDocument: lc.TextDocumentIdentifier | null
}>("aptos-analyzer/runFlycheck");

export type ServerStatusParams = {
    health: "ok" | "warning" | "error";
    quiescent: boolean;
    message?: string;
};

// export type SyntaxTreeParams = {
//     textDocument: lc.TextDocumentIdentifier;
//     range: lc.Range | null;
// };
export type ViewSyntaxTreeParams = { textDocument: lc.TextDocumentIdentifier };

export const viewSyntaxTree = new lc.RequestType<ViewSyntaxTreeParams, string, void>(
    "aptos-analyzer/viewSyntaxTree",
);
