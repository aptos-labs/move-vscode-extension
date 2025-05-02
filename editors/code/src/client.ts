import * as lc from "vscode-languageclient/node";
import * as vscode from "vscode";
import { WorkspaceEdit } from "vscode";
import * as Is from "vscode-languageclient/lib/common/utils/is";
import { prepareVSCodeConfig } from "./config";
import { assert, unwrapUndefinable } from "./util";

export async function createClient(
    traceOutputChannel: vscode.OutputChannel,
    outputChannel: vscode.OutputChannel,
    serverOptions: lc.ServerOptions,
): Promise<lc.LanguageClient> {

    const raMiddleware: lc.Middleware = {
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
        // Using custom handling of CodeActions to support action groups and snippet edits.
        // Note that this means we have to re-implement lazy edit resolving ourselves as well.
        async provideCodeActions(
            document: vscode.TextDocument,
            range: vscode.Range,
            context: vscode.CodeActionContext,
            token: vscode.CancellationToken,
            _next: lc.ProvideCodeActionsSignature,
        ) {
            const params: lc.CodeActionParams = {
                textDocument: client.code2ProtocolConverter.asTextDocumentIdentifier(document),
                range: client.code2ProtocolConverter.asRange(range),
                context: await client.code2ProtocolConverter.asCodeActionContext(context, token),
            };
            const callback = async (
                values: (lc.Command | lc.CodeAction)[] | null,
            ): Promise<(vscode.Command | vscode.CodeAction)[] | undefined> => {
                if (values === null) return undefined;
                const result: (vscode.CodeAction | vscode.Command)[] = [];
                const groups = new Map<string, { index: number; items: vscode.CodeAction[] }>();
                for (const item of values) {
                    // In our case we expect to get code edits only from diagnostics
                    if (lc.CodeAction.is(item)) {
                        assert(!item.command, "We don't expect to receive commands in CodeActions");
                        const action = await client.protocol2CodeConverter.asCodeAction(
                            item,
                            token,
                        );
                        result.push(action);
                        continue;
                    }
                    assert(
                        isCodeActionWithoutEditsAndCommands(item),
                        "We don't expect edits or commands here",
                    );
                    // eslint-disable-next-line @typescript-eslint/no-explicit-any
                    const kind = client.protocol2CodeConverter.asCodeActionKind((item as any).kind);
                    const action = new vscode.CodeAction(item.title, kind);
                    // eslint-disable-next-line @typescript-eslint/no-explicit-any
                    const group = (item as any).group;
                    action.command = {
                        command: "aptos-analyzer.resolveCodeAction",
                        title: item.title,
                        arguments: [item],
                    };

                    // Set a dummy edit, so that VS Code doesn't try to resolve this.
                    action.edit = new WorkspaceEdit();

                    if (group) {
                        let entry = groups.get(group);
                        if (!entry) {
                            entry = { index: result.length, items: [] };
                            groups.set(group, entry);
                            result.push(action);
                        }
                        entry.items.push(action);
                    } else {
                        result.push(action);
                    }
                }
                for (const [group, { index, items }] of groups) {
                    if (items.length === 1) {
                        result[index] = unwrapUndefinable(items[0]);
                    } else {
                        const action = new vscode.CodeAction(group);
                        const item = unwrapUndefinable(items[0]);
                        action.kind = item.kind;
                        action.command = {
                            command: "aptos-analyzer.applyActionGroup",
                            title: "",
                            arguments: [
                                items.map((item) => {
                                    return {
                                        label: item.title,
                                        arguments: item.command!.arguments![0],
                                    };
                                }),
                            ],
                        };

                        // Set a dummy edit, so that VS Code doesn't try to resolve this.
                        action.edit = new WorkspaceEdit();

                        result[index] = action;
                    }
                }
                return result;
            };
            return client
                .sendRequest(lc.CodeActionRequest.type, params, token)
                .then(callback, (_error) => undefined);
        },
    };


    const clientOptions: lc.LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'move' }],
        // initializationOptions,
        traceOutputChannel,
        outputChannel,
        middleware: raMiddleware,
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

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function isCodeActionWithoutEditsAndCommands(value: any): boolean {
    const candidate: lc.CodeAction = value;
    return (
        candidate &&
        Is.string(candidate.title) &&
        (candidate.diagnostics === void 0 ||
            Is.typedArray(candidate.diagnostics, lc.Diagnostic.is)) &&
        (candidate.kind === void 0 || Is.string(candidate.kind)) &&
        candidate.edit === void 0 &&
        candidate.command === void 0
    );
}