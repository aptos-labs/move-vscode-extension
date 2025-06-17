import vscode from "vscode";
import * as lsp_ext from "./lsp_ext";
import { Cmd, Ctx, CtxInit } from "./ctx";

export function analyzerStatus(ctx: CtxInit): Cmd {
    const tdcp = new (class implements vscode.TextDocumentContentProvider {
        readonly uri = vscode.Uri.parse("aptos-analyzer-status://status");
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
        vscode.workspace.registerTextDocumentContentProvider("aptos-analyzer-status", tdcp),
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
        const config = vscode.workspace.getConfiguration("aptos-analyzer");
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
            void vscode.window.showWarningMessage(`aptos-analyzer server is not running`);
            return;
        }
        void vscode.window.showInformationMessage(
            `aptos-analyzer version: ${ctx.serverVersion} [${ctx.serverPath}]`,
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
