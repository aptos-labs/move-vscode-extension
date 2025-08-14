// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

import * as vscode from "vscode";
import * as lsp_ext from "./lsp_ext";
import { Cmd, Ctx, CtxInit } from "./ctx";
import { LanguageClient } from "vscode-languageclient/node";
import * as lc from "vscode-languageclient";
import { createTaskFromRunnable } from "./run";
import { applyTextEdits } from "./snippets";

export function analyzerStatus(ctx: CtxInit): Cmd {
    const tdcp = new (class implements vscode.TextDocumentContentProvider {
        readonly uri = vscode.Uri.parse("aptos-lsp-status://status");
        readonly eventEmitter = new vscode.EventEmitter<vscode.Uri>();

        async provideTextDocumentContent(_uri: vscode.Uri): Promise<string> {
            if (!vscode.window.activeTextEditor) return "";
            const client = ctx.client;

            const params: lsp_ext.AnalyzerStatusParams = {};
            const doc = ctx.activeAptosEditor?.document;
            if (doc != null) {
                params.textDocument = client.code2ProtocolConverter.asTextDocumentIdentifier(doc);
            }
            return await client.sendRequest(lsp_ext.analyzerStatus, params);
        }

        get onDidChange(): vscode.Event<vscode.Uri> {
            return this.eventEmitter.event;
        }
    })();

    ctx.pushExtCleanup(
        vscode.workspace.registerTextDocumentContentProvider("aptos-lsp-status", tdcp),
    );

    return async () => {
        const document = await vscode.workspace.openTextDocument(tdcp.uri);
        tdcp.eventEmitter.fire(tdcp.uri);
        void (await vscode.window.showTextDocument(document, {
            viewColumn: vscode.ViewColumn.Two,
            preserveFocus: true,
        }));
    };
}

export function toggleLSPLogs(ctx: Ctx): Cmd {
    return async () => {
        const config = vscode.workspace.getConfiguration("move-on-aptos");
        const targetValue =
            config.get<string | undefined>("trace.server") === "verbose" ? undefined : "verbose";

        await config.update("trace.server", targetValue, vscode.ConfigurationTarget.Workspace);
        if (targetValue && ctx.client && ctx.client.traceOutputChannel) {
            ctx.client.traceOutputChannel.show();
        }
    };
}


export function syntaxTreeHideWhitespace(ctx: CtxInit): Cmd {
    return async () => {
        if (ctx.syntaxTreeProvider !== undefined) {
            await ctx.syntaxTreeProvider.toggleWhitespace();
        }
    };
}

export function syntaxTreeShowWhitespace(ctx: CtxInit): Cmd {
    return async () => {
        if (ctx.syntaxTreeProvider !== undefined) {
            await ctx.syntaxTreeProvider.toggleWhitespace();
        }
    };
}

export function serverVersion(ctx: CtxInit): Cmd {
    return async () => {
        if (!ctx.serverPath) {
            void vscode.window.showWarningMessage(`aptos-language-server is not running`);
            return;
        }
        void vscode.window.showInformationMessage(
            `aptos-language-server version: ${ctx.serverVersion} [${ctx.serverPath}]`,
        );
    };
}

export function openLogs(ctx: CtxInit): Cmd {
    return async () => {
        if (ctx.client.outputChannel) {
            ctx.client.outputChannel.show();
        }
    };
}

async function showReferencesImpl(
    client: LanguageClient | undefined,
    uri: string,
    position: lc.Position,
    locations: lc.Location[],
) {
    if (client) {
        await vscode.commands.executeCommand(
            "editor.action.showReferences",
            vscode.Uri.parse(uri),
            client.protocol2CodeConverter.asPosition(position),
            locations.map(client.protocol2CodeConverter.asLocation),
        );
    }
}

export function showReferences(ctx: CtxInit): Cmd {
    return async (uri: string, position: lc.Position, locations: lc.Location[]) => {
        await showReferencesImpl(ctx.client, uri, position, locations);
    };
}

export function gotoLocation(ctx: CtxInit): Cmd {
    return async (locationLink: lc.LocationLink) => {
        const client = ctx.client;
        const uri = client.protocol2CodeConverter.asUri(locationLink.targetUri);
        let range = client.protocol2CodeConverter.asRange(locationLink.targetSelectionRange);
        // collapse the range to a cursor position
        range = range.with({ end: range.start });

        await vscode.window.showTextDocument(uri, { selection: range });
    };
}

export function runSingle(ctx: CtxInit): Cmd {
    return async (runnable: lsp_ext.Runnable) => {
        const editor = ctx.activeAptosEditor;
        if (!editor) return;

        const task = await createTaskFromRunnable(runnable, ctx.config);
        task.group = vscode.TaskGroup.Test;
        task.presentationOptions = {
            reveal: vscode.TaskRevealKind.Always,
            panel: vscode.TaskPanelKind.Dedicated,
            clear: true,
        };

        return vscode.tasks.executeTask(task);
    };
}

export function organizeImports(ctx: CtxInit): Cmd {
    return async () => {
        const editor = ctx.activeAptosEditor;
        if (!editor) return;
        const client = ctx.client;

        const lcEdits = await client.sendRequest(lsp_ext.organizeImports, {
            textDocument: client.code2ProtocolConverter.asTextDocumentIdentifier(editor.document),
        });
        if (!lcEdits) return;

        const edits = await client.protocol2CodeConverter.asTextEdits(lcEdits);
        await applyTextEdits(editor, edits);
    };
}
