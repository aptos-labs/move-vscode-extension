import * as vscode from 'vscode';
import {IndentAction} from 'vscode';
import * as lc from "vscode-languageclient/node";
import {Configuration} from "./config";
import {log} from "./util";

export class Ctx {
    private _client: lc.LanguageClient | undefined;

    private commandFactories: Record<string, CommandFactory>;
    private commandDisposables: Disposable[];

    get client(): lc.LanguageClient | undefined {
        return this._client;
    }

    constructor(
        private readonly extensionContext: Readonly<vscode.ExtensionContext>,
        readonly config: Readonly<Configuration>,
        commandFactories: Record<string, CommandFactory>,
        client: lc.LanguageClient | undefined = undefined,
    ) {
        this._client = client;

        this.commandFactories = commandFactories;
        this.commandDisposables = [];

        this.updateCommands("disable");
    }

    /**
     * Sets up additional language configuration that's impossible to do via a
     * separate language-configuration.json file. See [1] for more information.
     *
     * This code originates from [2](vscode-rust).
     *
     * [1]: https://github.com/Microsoft/vscode/issues/11514#issuecomment-244707076
     * [2]: https://github.com/rust-lang/vscode-rust/blob/660b412701fe2ea62fad180c40ee4f8a60571c61/src/extension.ts#L287:L287
     */
    configureLanguage(): void {
        const disposable = vscode.languages.setLanguageConfiguration('move', {
            onEnterRules: [
                {
                    // Doc single-line comment
                    // e.g. ///|
                    beforeText: /^\s*\/{3}.*$/,
                    action: {indentAction: IndentAction.None, appendText: '/// '},
                },
                {
                    // Parent doc single-line comment
                    // e.g. //!|
                    beforeText: /^\s*\/{2}!.*$/,
                    action: {indentAction: IndentAction.None, appendText: '//! '},
                },
            ],
        });
        this.extensionContext.subscriptions.push(disposable);
    }

    async createClient(): Promise<lc.LanguageClient> {
        const newEnv = Object.assign({}, process.env, this.config.serverExtraEnv);
        const executable: lc.Executable = {
            command: this.config.serverPath,
            options: {shell: true, env: newEnv},
        };
        const serverOptions: lc.ServerOptions = {
            run: executable,
            debug: executable,
        };

        // The vscode-languageclient module reads a configuration option named
        // "<extension-name>.trace.server" to determine whether to log messages. If a trace output
        // channel is specified, these messages are printed there, otherwise they appear in the
        // output channel that it automatically created by the `LanguageClient` (in this extension,
        // that is 'Move Language Server'). For more information, see:
        // https://code.visualstudio.com/api/language-extensions/language-server-extension-guide#logging-support-for-language-server
        const traceOutputChannel = vscode.window.createOutputChannel(
            'Aptos Analyzer Trace',
        );
        vscode.workspace.onDidChangeConfiguration(
            async (_) => {
                await this.client?.sendNotification(lc.DidChangeConfigurationNotification.type, {
                    settings: "",
                });
            },
            null,
        )
        const clientOptions: lc.LanguageClientOptions = {
            documentSelector: [{scheme: 'file', language: 'move'}],
            traceOutputChannel,
        };
        this._client = new lc.LanguageClient(
            'aptos-analyzer',
            'Aptos Analyzer Language Server',
            serverOptions,
            clientOptions,
        );

        return this._client;
    }

    async start(): Promise<void> {
        log.info("Starting language client");

        const client = await this.createClient();
        await client.start();

        this.updateCommands();
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
        // this.clientSubscriptions?.forEach((disposable) => disposable.dispose());
        // this.clientSubscriptions = [];
        await this._client?.dispose();
        // this._serverPath = undefined;
        this._client = undefined;
    }

    private updateCommands(forceDisable?: "disable") {
        this.commandDisposables.forEach((disposable) => disposable.dispose());
        this.commandDisposables = [];

        const clientRunning = (!forceDisable && this._client?.isRunning()) ?? false;
        const isClientRunning = function (_ctx: Ctx): _ctx is CtxInit {
            return clientRunning;
        };

        for (const [name, factory] of Object.entries(this.commandFactories)) {
            const fullName = `aptos-analyzer.${name}`;
            let callback;
            if (isClientRunning(this)) {
                // we asserted that `client` is defined
                callback = factory.enabled(this);
            } else if (factory.disabled) {
                callback = factory.disabled(this);
            } else {
                callback = () =>
                    vscode.window.showErrorMessage(
                        `command ${fullName} failed: aptos-analyzer server is not running`,
                    );
            }

            this.commandDisposables.push(vscode.commands.registerCommand(fullName, callback));
        }
    }

    pushExtCleanup(d: Disposable) {
        this.extensionContext.subscriptions.push(d);
    }
}

export type CommandFactory = {
    enabled: (ctx: CtxInit) => Cmd;
    disabled?: (ctx: Ctx) => Cmd;
};

export type CtxInit = Ctx & {
    readonly client: lc.LanguageClient;
};

export interface Disposable {
    dispose(): void;
}

export type Cmd = (...args: any[]) => unknown;
