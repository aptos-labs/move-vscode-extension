// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

import * as vscode from 'vscode';
import * as lc from "vscode-languageclient/node";
import { Config } from "./config";
import { AptosEditor, isAptosDocument, isAptosEditor, isMoveTomlEditor, LazyOutputChannel, log } from "./util";
import { SyntaxElement, SyntaxTreeProvider } from "./syntax_tree_provider";
import { createClient } from "./client";
import { bootstrap } from "./bootstrap";
import * as lsp_ext from "./lsp_ext";
import { text } from "node:stream/consumers";
import { spawn } from "node:child_process";


// We only support local folders, not eg. Live Share (`vlsl:` scheme), so don't activate if
// only those are in use. We use "Empty" to represent these scenarios
// (r-a still somewhat works with Live Share, because commands are tunneled to the host)

// note: required version is >1.2.1, and 1.2.4 is just latest at this time
const MOVEFMT_REQUIRED_VERSION = "1.2.4";

export type Workspace =
    | { kind: "Empty" }
    | { kind: "Workspace Folder" }
    | { kind: "Detached Files"; files: vscode.TextDocument[] };

export function fetchWorkspace(): Workspace {
    const folders = (vscode.workspace.workspaceFolders || []).filter(
        (folder) => folder.uri.scheme === "file",
    );
    // const aptosDocuments = vscode.workspace.textDocuments.filter((document) =>
    //     isAptosDocument(document),
    // );
    return folders.length === 0 ? { kind: "Empty" } : { kind: "Workspace Folder" };
    // return folders.length === 0
    //     ? aptosDocuments.length === 0
    //         ? { kind: "Empty" }
    //         : { kind: "Detached Files", files: aptosDocuments }
    //     : { kind: "Workspace Folder" };
}

export type CommandFactory = {
    enabled: (ctx: CtxInit) => Cmd;
    disabled?: (ctx: Ctx) => Cmd;
};

export type CtxInit = Ctx & {
    readonly client: lc.LanguageClient;
};

export class Ctx {
    readonly statusBar: vscode.StatusBarItem;
    readonly config: Config;
    readonly version: string;
    readonly workspace: Workspace;

    private _client: lc.LanguageClient | undefined;
    private _serverPath: string | undefined;
    private traceOutputChannel: vscode.OutputChannel | undefined;
    private outputChannel: vscode.OutputChannel | undefined;
    private clientSubscriptions: Disposable[];
    private commandFactories: Record<string, CommandFactory>;
    private commandDisposables: Disposable[];

    private _syntaxTreeProvider: SyntaxTreeProvider | undefined;
    private _syntaxTreeView: vscode.TreeView<SyntaxElement> | undefined;

    private lastStatus: lsp_ext.ServerStatusParams | { health: "stopped" } = { health: "stopped" };
    private _serverVersion: string;
    private statusBarActiveEditorListener: Disposable;

    get serverPath(): string | undefined {
        return this._serverPath;
    }

    get serverVersion(): string | undefined {
        return this._serverVersion;
    }

    get client(): lc.LanguageClient | undefined {
        return this._client;
    }

    get syntaxTreeView() {
        return this._syntaxTreeView;
    }

    get syntaxTreeProvider() {
        return this._syntaxTreeProvider;
    }

    constructor(
        private readonly extCtx: Readonly<vscode.ExtensionContext>,
        commandFactories: Record<string, CommandFactory>,
        workspace: Workspace,
    ) {
        extCtx.subscriptions.push(this);
        this.version = extCtx.extension.packageJSON.version ?? "<unknown>";
        this._serverVersion = "<not running>";
        this.config = new Config(extCtx.subscriptions);

        this.statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left);
        this.updateStatusBarVisibility(vscode.window.activeTextEditor);
        this.statusBarActiveEditorListener = vscode.window.onDidChangeActiveTextEditor((editor) =>
            this.updateStatusBarVisibility(editor),
        );

        this.workspace = workspace;
        this.clientSubscriptions = [];
        this.commandFactories = commandFactories;
        this.commandDisposables = [];

        this.updateCommands("disable");
        this.setServerStatus({ health: "stopped" });
    }

    dispose() {
        this.config.dispose();
        this.statusBar.dispose();
        this.statusBarActiveEditorListener.dispose();
        void this.disposeClient();
        this.commandDisposables.forEach((disposable) => disposable.dispose());
    }

    async onWorkspaceFolderChanges() {
        const workspace = fetchWorkspace();
        if (workspace.kind === "Detached Files" && this.workspace.kind === "Detached Files") {
            if (workspace.files !== this.workspace.files) {
                if (this.client?.isRunning()) {
                    // Ideally we wouldn't need to tear down the server here, but currently detached files
                    // are only specified at server start
                    await this.stopAndDispose();
                    await this.start();
                }
                return;
            }
        }
        if (workspace.kind === "Workspace Folder" && this.workspace.kind === "Workspace Folder") {
            return;
        }
        if (workspace.kind === "Empty") {
            await this.stopAndDispose();
            return;
        }
        if (this.client?.isRunning()) {
            await this.restart();
        }
    }

    private async getOrCreateClient(): Promise<lc.LanguageClient | undefined> {
        if (this.workspace.kind === "Empty") {
            return;
        }

        // The vscode-languageclient module reads a configuration option named
        // "<extension-name>.trace.server" to determine whether to log messages. If a trace output
        // channel is specified, these messages are printed there, otherwise they appear in the
        // output channel that it automatically created by the `LanguageClient` (in this extension,
        // that is 'Move Language Server'). For more information, see:
        // https://code.visualstudio.com/api/language-extensions/language-server-extension-guide#logging-support-for-language-server
        if (!this.traceOutputChannel) {
            this.traceOutputChannel = new LazyOutputChannel("Move-on-Aptos LSP Trace");
            this.pushExtCleanup(this.traceOutputChannel);
        }
        if (!this.outputChannel) {
            this.outputChannel = vscode.window.createOutputChannel("Move-on-Aptos Language Server");
            this.pushExtCleanup(this.outputChannel);
        }

        if (!this._client) {
            this._serverPath = await this.bootstrap();
            text(spawn(this._serverPath, ["--version"]).stdout.setEncoding("utf-8")).then(
                (data) => {
                    const prefix = `aptos-language-server `;
                    this._serverVersion = data
                        .slice(data.startsWith(prefix) ? prefix.length : 0)
                        .trim();
                    this.refreshServerStatus();
                },
                (_) => {
                    this._serverVersion = "<unknown>";
                    this.refreshServerStatus();
                },
            );
            const newEnv = Object.assign({}, process.env, this.config.serverExtraEnv);
            const run: lc.Executable = {
                command: this._serverPath,
                args: ["lsp-server"],
                options: { env: newEnv },
            };
            const serverOptions: lc.ServerOptions = {
                run,
                debug: run,
            };
            this._client = await createClient(this.traceOutputChannel, this.outputChannel, serverOptions)
            this.pushClientCleanup(
                this._client.onNotification(lsp_ext.serverStatus, (params) =>
                    this.setServerStatus(params),
                ),
            );
            this.pushClientCleanup(
                this._client.onNotification(lsp_ext.openServerLogs, () => {
                    this.outputChannel!.show();
                }),
            );
            this.pushClientCleanup(
                this._client.onNotification(lsp_ext.movefmtVersionError, async (params) => {
                    const aptosPath = params.aptosPath;
                    const warningMessage = `movefmt error: ${params.message}`;
                    if (aptosPath === undefined) {
                        await vscode.window.showErrorMessage(
                            `${warningMessage}. Configure 'move-on-aptos.aptosPath' to fetch it from the editor`
                        );
                        return;
                    }
                    const updateLabel = "Run `aptos update` in Terminal";
                    const selected =
                        await vscode.window.showErrorMessage(warningMessage, updateLabel);
                    if (selected === updateLabel) {
                        const terminal = vscode.window.createTerminal(`Update Movefmt`);
                        terminal.sendText(`${aptosPath} update movefmt --target-version ${MOVEFMT_REQUIRED_VERSION}`);
                        terminal.show(false)
                    }
                }),
            );
        }
        return this._client;
    }

    private async bootstrap(): Promise<string> {
        return bootstrap(this.extCtx, this.config).catch((err) => {
            let message = "bootstrap error. ";

            message +=
                'See the logs in "OUTPUT > Aptos Analyzer Client" (should open automatically).';
            message +=
                'To enable verbose logs, click the gear icon in the "OUTPUT" tab and select "Debug".';

            log.error("Bootstrap error", err);
            throw new Error(message);
        });
    }

    async start(): Promise<void> {
        log.info("Starting language client");
        const client = await this.getOrCreateClient();
        if (!client) {
            return;
        }
        await client.start();
        this.updateCommands();

        if (this.config.showSyntaxTree) {
            this.prepareSyntaxTreeView(client);
        }
    }

    private prepareSyntaxTreeView(client: lc.LanguageClient) {
        const ctxInit: CtxInit = {
            ...this,
            client: client,
        };
        this._syntaxTreeProvider = new SyntaxTreeProvider(ctxInit);
        this._syntaxTreeView = vscode.window.createTreeView("aptosSyntaxTree", {
            treeDataProvider: this._syntaxTreeProvider,
            showCollapseAll: true,
        });

        this.pushExtCleanup(this._syntaxTreeView);

        vscode.window.onDidChangeActiveTextEditor(async () => {
            if (this.syntaxTreeView?.visible) {
                await this.syntaxTreeProvider?.refresh();
            }
        });

        vscode.workspace.onDidChangeTextDocument(async (e) => {
            if (
                vscode.window.activeTextEditor?.document !== e.document ||
                e.contentChanges.length === 0
            ) {
                return;
            }

            if (this.syntaxTreeView?.visible) {
                await this.syntaxTreeProvider?.refresh();
            }
        });

        vscode.window.onDidChangeTextEditorSelection(async (e) => {
            if (!this.syntaxTreeView?.visible || !isAptosEditor(e.textEditor)) {
                return;
            }

            const selection = e.selections[0];
            if (selection === undefined) {
                return;
            }

            const result = this.syntaxTreeProvider?.getElementByRange(selection);
            if (result !== undefined) {
                await this.syntaxTreeView?.reveal(result);
            }
        });

        this._syntaxTreeView.onDidChangeVisibility(async (e) => {
            if (e.visible) {
                await this.syntaxTreeProvider?.refresh();
            }
        });
    }

    async restart() {
        // FIXME: We should re-use the client, that is ctx.deactivate() if none of the configs have changed
        await this.stopAndDispose();
        await this.start();
    }

    async stopAndDispose() {
        if (!this._client) {
            return;
        }
        log.info("Disposing language client");
        this.updateCommands("disable");
        // we give the server 100ms to stop gracefully
        await this.client?.stop(100).catch((_) => {
        });
        await this.disposeClient();
    }

    private async disposeClient() {
        this.clientSubscriptions?.forEach((disposable) => disposable.dispose());
        this.clientSubscriptions = [];
        await this._client?.dispose();
        this._serverPath = undefined;
        this._client = undefined;
    }

    get activeAptosEditor(): AptosEditor | undefined {
        const editor = vscode.window.activeTextEditor;
        return editor && isAptosEditor(editor) ? editor : undefined;
    }

    get activeMoveTomlEditor(): AptosEditor | undefined {
        const editor = vscode.window.activeTextEditor;
        return editor && isMoveTomlEditor(editor) ? editor : undefined;
    }

    get extensionPath(): string {
        return this.extCtx.extensionPath;
    }

    get subscriptions(): Disposable[] {
        return this.extCtx.subscriptions;
    }

    private updateCommands(forceDisable?: "disable") {
        this.commandDisposables.forEach((disposable) => disposable.dispose());
        this.commandDisposables = [];

        const clientRunning = (!forceDisable && this._client?.isRunning()) ?? false;
        const isClientRunning = function (_ctx: Ctx): _ctx is CtxInit {
            return clientRunning;
        };

        for (const [name, factory] of Object.entries(this.commandFactories)) {
            const fullName = `move-on-aptos.${name}`;
            let callback;
            if (isClientRunning(this)) {
                // we asserted that `client` is defined
                callback = factory.enabled(this);
            } else if (factory.disabled) {
                callback = factory.disabled(this);
            } else {
                callback = () =>
                    vscode.window.showErrorMessage(
                        `command ${fullName} failed: aptos-language-server is not running`,
                    );
            }

            this.commandDisposables.push(vscode.commands.registerCommand(fullName, callback));
        }
    }

    setServerStatus(status: lsp_ext.ServerStatusParams | { health: "stopped" }) {
        this.lastStatus = status;
        this.updateStatusBarItem();
    }

    refreshServerStatus() {
        this.updateStatusBarItem();
    }

    private updateStatusBarItem() {
        let icon = "";
        const status = this.lastStatus;
        const statusBar = this.statusBar;
        statusBar.tooltip = new vscode.MarkdownString("", true);
        statusBar.tooltip.isTrusted = true;
        switch (status.health) {
            case "ok":
                statusBar.color = undefined;
                statusBar.backgroundColor = undefined;
                if (this.config.statusBarClickAction === "stopServer") {
                    statusBar.command = "move-on-aptos.stopServer";
                } else {
                    statusBar.command = "move-on-aptos.openLogs";
                }
                void this.syntaxTreeProvider?.refresh();
                break;
            case "warning":
                statusBar.color = new vscode.ThemeColor("statusBarItem.warningForeground");
                statusBar.backgroundColor = new vscode.ThemeColor(
                    "statusBarItem.warningBackground",
                );
                statusBar.command = "move-on-aptos.openLogs";
                icon = "$(warning) ";
                break;
            case "error":
                statusBar.color = new vscode.ThemeColor("statusBarItem.errorForeground");
                statusBar.backgroundColor = new vscode.ThemeColor("statusBarItem.errorBackground");
                statusBar.command = "move-on-aptos.openLogs";
                icon = "$(error) ";
                break;
            case "stopped":
                statusBar.tooltip.appendText("Server is stopped");
                statusBar.tooltip.appendMarkdown(
                    "\n\n[Start server](command:move-on-aptos.startServer)",
                );
                statusBar.color = new vscode.ThemeColor("statusBarItem.warningForeground");
                statusBar.backgroundColor = new vscode.ThemeColor(
                    "statusBarItem.warningBackground",
                );
                statusBar.command = "move-on-aptos.startServer";
                statusBar.text = "$(stop-circle) move-on-aptos";
                return;
        }
        if (status.message) {
            statusBar.tooltip.appendMarkdown(status.message);
        }
        if (statusBar.tooltip.value) {
            statusBar.tooltip.appendMarkdown("\n\n---\n\n");
        }

        statusBar.tooltip.appendMarkdown(
            `[Extension Info](command:move-on-aptos.serverVersion "Show version and server binary info"): Version ${this.version}, Server Version ${this._serverVersion}\n\n` +
            `---\n\n` +
            `[$(terminal) Open Logs](command:move-on-aptos.openLogs "Open the server logs")\n\n` +
            // `[$(refresh) Reload Workspace](command:rust-analyzer.reloadWorkspace "Reload and rediscover workspaces")\n\n` +
            `[$(stop-circle) Stop server](command:move-on-aptos.stopServer "Stop the server")\n\n` +
            `[$(debug-restart) Restart server](command:move-on-aptos.restartServer "Restart the server")`,
        );
        if (!status.quiescent) icon = "$(loading~spin) ";
        statusBar.text = `${icon}move-on-aptos`;
    }

    private updateStatusBarVisibility(editor: vscode.TextEditor | undefined) {
        const showStatusBar = this.config.statusBarShowStatusBar;
        if (showStatusBar == null || showStatusBar === "never") {
            this.statusBar.hide();
        } else if (showStatusBar === "always") {
            this.statusBar.show();
        } else {
            const documentSelector = showStatusBar.documentSelector;
            if (editor != null && vscode.languages.match(documentSelector, editor.document) > 0) {
                this.statusBar.show();
            } else {
                this.statusBar.hide();
            }
        }
    }

    pushExtCleanup(d: Disposable) {
        this.extCtx.subscriptions.push(d);
    }

    pushClientCleanup(d: Disposable) {
        this.clientSubscriptions.push(d);
    }
}

export interface Disposable {
    dispose(): void;
}

export type Cmd = (...args: any[]) => unknown;
