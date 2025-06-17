import * as lc from "vscode-languageclient/node";
import * as vscode from "vscode";
import { prepareVSCodeConfig } from "./config";

export async function createClient(
    traceOutputChannel: vscode.OutputChannel,
    outputChannel: vscode.OutputChannel,
    serverOptions: lc.ServerOptions,
): Promise<lc.LanguageClient> {

    const lspMiddleware: lc.Middleware = {
        workspace: {
            // HACK: This is a workaround, when the client has been disposed, VSCode
            // continues to emit events to the client and the default one for this event
            // attempt to restart the client for no reason
            async didChangeWatchedFile(event, next) {
                if (client.isRunning()) {
                    await next(event);
                }
            },
            async configuration(
                params: lc.ConfigurationParams,
                token: vscode.CancellationToken,
                next: lc.ConfigurationRequest.HandlerSignature,
            ) {
                const resp = await next(params, token);
                if (resp && Array.isArray(resp)) {
                    return resp.map((val) => {
                        return prepareVSCodeConfig(val);
                    });
                } else {
                    return resp;
                }
            },
        },
    };


    const clientOptions: lc.LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'move' }],
        // initializationOptions,
        traceOutputChannel,
        outputChannel,
        middleware: lspMiddleware,
        markdown: {
            supportHtml: true,
        },
    };

    // const newEnv = Object.assign({}, process.env, this.config.serverExtraEnv);
    // const executable: lc.Executable = {
    //     command: this.config.serverPath,
    //     options: {shell: true, env: newEnv},
    // };
    // const serverOptions: lc.ServerOptions = {
    //     run: executable,
    //     debug: executable,
    // };

    // The vscode-languageclient module reads a configuration option named
    // "<extension-name>.trace.server" to determine whether to log messages. If a trace output
    // channel is specified, these messages are printed there, otherwise they appear in the
    // output channel that it automatically created by the `LanguageClient` (in this extension,
    // that is 'Move Language Server'). For more information, see:
    // https://code.visualstudio.com/api/language-extensions/language-server-extension-guide#logging-support-for-language-server
    // const traceOutputChannel = vscode.window.createOutputChannel(
    //     'Aptos Analyzer Trace',
    // );
    // vscode.workspace.onDidChangeConfiguration(
    //     async (_) => {
    //         await this.client?.sendNotification(lc.DidChangeConfigurationNotification.type, {
    //             settings: "",
    //         });
    //     },
    //     null,
    // )
    // const clientOptions: lc.LanguageClientOptions = {
    //     documentSelector: [{scheme: 'file', language: 'move'}],
    //     traceOutputChannel,
    // };
    // this._client = new lc.LanguageClient(
    //     'aptos-analyzer',
    //     'Aptos Analyzer Language Server',
    //     serverOptions,
    //     clientOptions,
    // );

    const client = new lc.LanguageClient(
        'aptos-analyzer',
        'Aptos Analyzer Language Server',
        serverOptions,
        clientOptions,
    );

    // To turn on all proposed features use: client.registerProposedFeatures();
    client.registerFeature(new ExperimentalFeatures(/*config*/));

    return client;
    // return this._client;
}

class ExperimentalFeatures implements lc.StaticFeature {
    // private readonly testExplorer: boolean;
    //
    // constructor(config: Configuration) {
    //     this.testExplorer = config.testExplorer || false;
    // }

    getState(): lc.FeatureState {
        return { kind: "static" };
    }

    fillClientCapabilities(capabilities: lc.ClientCapabilities): void {
        capabilities.experimental = {
            snippetTextEdit: true,
            codeActionGroup: true,
            // hoverActions: true,
            serverStatusNotification: true,
            // colorDiagnosticOutput: true,
            openServerLogs: true,
            // localDocs: true,
            // testExplorer: this.testExplorer,
            // commands: {
            //     commands: [
            //         "rust-analyzer.runSingle",
            //         "rust-analyzer.debugSingle",
            //         "rust-analyzer.showReferences",
            //         "rust-analyzer.gotoLocation",
            //         "rust-analyzer.triggerParameterHints",
            //         "rust-analyzer.rename",
            //     ],
            // },
            ...capabilities.experimental,
        };
    }

    initialize(
        _capabilities: lc.ServerCapabilities,
        _documentSelector: lc.DocumentSelector | undefined,
    ): void {
    }

    dispose(): void {
    }

    clear(): void {
    }
}
