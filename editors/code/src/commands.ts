// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

import * as net from "net";
import * as vscode from "vscode";
import * as lsp_ext from "./lsp_ext";
import { Cmd, Ctx, CtxInit } from "./ctx";
import { LanguageClient } from "vscode-languageclient/node";
import * as lc from "vscode-languageclient";
import { createTaskFromRunnable } from "./run";
import { applyTextEdits } from "./snippets";

function delay(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
}

export function waitForPort(port: number, timeoutMs: number = 10000, intervalMs: number = 200): Promise<void> {
    return new Promise((resolve, reject) => {
        const deadline = Date.now() + timeoutMs;
        function tryConnect() {
            const sock = new net.Socket();
            sock.once("connect", () => {
                sock.destroy();
                resolve();
            });
            sock.once("error", () => {
                sock.destroy();
                if (Date.now() >= deadline) {
                    reject(new Error(`aptos-dap did not start within ${timeoutMs}ms on port ${port}`));
                } else {
                    setTimeout(tryConnect, intervalMs);
                }
            });
            sock.connect(port, "127.0.0.1");
        }
        tryConnect();
    });
}

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

export function runTest(ctx: CtxInit): Cmd {
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

export function debugTest(_ctx: CtxInit): Cmd {
    return async (runnable: lsp_ext.Runnable) => {
        const args = runnable.args;
        const filterIdx = args.args.indexOf("--filter");
        const testFilter = filterIdx >= 0 ? args.args[filterIdx + 1] : undefined;
        if (!testFilter) {
            vscode.window.showErrorMessage("Could not determine test filter from runnable.");
            return;
        }

        await vscode.debug.startDebugging(undefined, {
            type: "aptos-move-test",
            request: "launch",
            name: `Debug ${testFilter}`,
            testFilter,
            packagePath: args.packageRoot,
        });
    };
}

export function debugTransaction(_ctx: CtxInit): Cmd {
    return async (runnable: lsp_ext.Runnable) => {
        const wsFolder = vscode.workspace.workspaceFolders?.[0];
        if (!wsFolder) {
            vscode.window.showErrorMessage("No workspace folder open.");
            return;
        }

        const networkStr = await vscode.window.showQuickPick(
            ["mainnet", "testnet", "devnet"],
            { placeHolder: "Select network" },
        );
        if (!networkStr) return;

        const fqName = runnable.label.replace(/^txn /, "");
        const parts = fqName.split("::");
        const moduleFn = parts.slice(1).join("::");

        const sdk = await import("@aptos-labs/ts-sdk");
        const networkMap: Record<string, typeof sdk.Network[keyof typeof sdk.Network]> = {
            mainnet: sdk.Network.MAINNET,
            testnet: sdk.Network.TESTNET,
            devnet: sdk.Network.DEVNET,
        };
        const aptos = new sdk.Aptos(new sdk.AptosConfig({ network: networkMap[networkStr] }));
        const namedAddress = parts[0];
        let resolvedHexAddress: string | undefined;

        const txnIdStr = await vscode.window.showInputBox({
            title: "Transaction version to replay",
            prompt: `Must be a ${fqName}`,
            placeHolder: "e.g. 123456789",
            validateInput: async (v) => {
                if (!/^\d+$/.test(v)) return "Must be a number";
                try {
                    const txn = await aptos.getTransactionByVersion({
                        ledgerVersion: Number(v),
                    });
                    if (!sdk.isUserTransactionResponse(txn)) {
                        return `Transaction ${v} is not a user transaction`;
                    }
                    const payload = txn.payload;
                    if (payload.type !== "entry_function_payload") {
                        return `Transaction ${v} is not an entry function call`;
                    }
                    const onChainFn = (payload as { function: string }).function;
                    const onChainParts = onChainFn.split("::");
                    const onChainModuleFn = onChainParts.slice(1).join("::");
                    if (onChainModuleFn !== moduleFn) {
                        return `Transaction calls ${onChainFn}, expected *::${moduleFn}`;
                    }
                    resolvedHexAddress = onChainParts[0];
                    return null;
                } catch {
                    return `Could not fetch transaction ${v}`;
                }
            },
        });
        if (!txnIdStr) return;

        const namedAddresses: Record<string, string> = {};
        if (resolvedHexAddress) {
            namedAddresses[namedAddress] = resolvedHexAddress;
        }

        const wsRoot = wsFolder.uri.fsPath;
        const allRoots = runnable.args.depRoots ?? [runnable.args.packageRoot];
        const useLocalPackages = allRoots
            .map((root) =>
                root.startsWith(wsRoot)
                    ? "${workspaceFolder}" + root.slice(wsRoot.length)
                    : root,
            )
            .sort((a, b) => {
                const aLocal = a.startsWith("${workspaceFolder}");
                const bLocal = b.startsWith("${workspaceFolder}");
                if (aLocal === bLocal) return 0;
                return aLocal ? 1 : -1;
            });

        const newConfig = {
            name: `Debug transaction ${moduleFn}`,
            type: "aptos-move-replay",
            request: "launch",
            network: networkStr,
            txnId: Number(txnIdStr),
            useLocalPackages,
            _note: `'${namedAddress}' address is inferred from the transaction. Add other named addresses if their on-chain values are different from ones in the source code.`,
            namedAddresses,
        };

        const launchConfig = vscode.workspace.getConfiguration("launch", wsFolder.uri);
        const configurations: vscode.DebugConfiguration[] =
            launchConfig.get("configurations") ?? [];

        const existingNames = new Set(configurations.map((c) => c.name));
        let name = newConfig.name;
        let counter = 1;
        while (existingNames.has(name)) {
            name = `${newConfig.name} (${counter})`;
            counter++;
        }
        newConfig.name = name;

        configurations.push(newConfig);

        await launchConfig.update("configurations", configurations);

        const launchJsonUri = vscode.Uri.joinPath(wsFolder.uri, ".vscode", "launch.json");
        const doc = await vscode.workspace.openTextDocument(launchJsonUri);
        const editor = await vscode.window.showTextDocument(doc);

        const text = doc.getText();
        const nameOffset = text.indexOf(`"name": "${newConfig.name}"`);
        if (nameOffset >= 0) {
            const pos = doc.positionAt(nameOffset);
            editor.selection = new vscode.Selection(pos, pos);
            editor.revealRange(new vscode.Range(pos, pos), vscode.TextEditorRevealType.InCenter);
        }

        await vscode.debug.startDebugging(wsFolder, newConfig.name);
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
