import * as lc from "vscode-languageclient/node";
import * as vscode from "vscode";
import * as Is from "vscode-languageclient/lib/common/utils/is";

export async function createClient(
    traceOutputChannel: vscode.OutputChannel,
    // outputChannel: vscode.OutputChannel,
    // initializationOptions: vscode.WorkspaceConfiguration,
    serverOptions: lc.ServerOptions,
): Promise<lc.LanguageClient> {

    const clientOptions: lc.LanguageClientOptions = {
        documentSelector: [{scheme: 'file', language: 'move'}],
        // initializationOptions,
        traceOutputChannel,
        // outputChannel,
        // middleware: raMiddleware,
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
    return client;
    // return this._client;
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