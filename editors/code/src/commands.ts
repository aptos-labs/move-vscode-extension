import vscode from "vscode";
import * as lc from "vscode-languageclient";
import * as lsp_ext from "./lsp_ext";
import {Cmd, Ctx, CtxInit} from "./ctx";
import {applySnippetWorkspaceEdit, SnippetTextDocumentEdit} from "./snippets";

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

export function applyActionGroup(_ctx: CtxInit): Cmd {
    return async (actions: { label: string; arguments: lc.CodeAction }[]) => {
        const selectedAction = await vscode.window.showQuickPick(actions);
        if (!selectedAction) return;
        await vscode.commands.executeCommand(
            "aptos-analyzer.resolveCodeAction",
            selectedAction.arguments,
        );
    };
}

export function resolveCodeAction(ctx: CtxInit): Cmd {
    return async (params: lc.CodeAction) => {
        const client = ctx.client;
        params.command = undefined;
        const item = await client.sendRequest(lc.CodeActionResolveRequest.type, params);
        if (!item?.edit) {
            return;
        }
        const itemEdit = item.edit;
        // filter out all text edits and recreate the WorkspaceEdit without them so we can apply
        // snippet edits on our own
        const lcFileSystemEdit = {
            ...itemEdit,
            documentChanges: itemEdit.documentChanges?.filter((change) => "kind" in change),
        };
        const fileSystemEdit =
            await client.protocol2CodeConverter.asWorkspaceEdit(lcFileSystemEdit);
        await vscode.workspace.applyEdit(fileSystemEdit);

        // replace all text edits so that we can convert snippet text edits into `vscode.SnippetTextEdit`s
        // FIXME: this is a workaround until vscode-languageclient supports doing the SnippeTextEdit conversion itself
        // also need to carry the snippetTextDocumentEdits separately, since we can't retrieve them again using WorkspaceEdit.entries
        const [workspaceTextEdit, snippetTextDocumentEdits] = asWorkspaceSnippetEdit(ctx, itemEdit);
        await applySnippetWorkspaceEdit(workspaceTextEdit, snippetTextDocumentEdits);
        if (item.command != null) {
            await vscode.commands.executeCommand(item.command.command, item.command.arguments);
        }
    };
}

function asWorkspaceSnippetEdit(
    ctx: CtxInit,
    item: lc.WorkspaceEdit,
): [vscode.WorkspaceEdit, SnippetTextDocumentEdit[]] {
    const client = ctx.client;

    // partially borrowed from https://github.com/microsoft/vscode-languageserver-node/blob/295aaa393fda8ecce110c38880a00466b9320e63/client/src/common/protocolConverter.ts#L1060-L1101
    const result = new vscode.WorkspaceEdit();

    if (item.documentChanges) {
        const snippetTextDocumentEdits: SnippetTextDocumentEdit[] = [];

        for (const change of item.documentChanges) {
            if (lc.TextDocumentEdit.is(change)) {
                const uri = client.protocol2CodeConverter.asUri(change.textDocument.uri);
                const snippetTextEdits: (vscode.TextEdit | vscode.SnippetTextEdit)[] = [];

                for (const edit of change.edits) {
                    if (
                        "insertTextFormat" in edit &&
                        edit.insertTextFormat === lc.InsertTextFormat.Snippet
                    ) {
                        // is a snippet text edit
                        snippetTextEdits.push(
                            new vscode.SnippetTextEdit(
                                client.protocol2CodeConverter.asRange(edit.range),
                                new vscode.SnippetString(edit.newText),
                            ),
                        );
                    } else {
                        // always as a text document edit
                        snippetTextEdits.push(
                            vscode.TextEdit.replace(
                                client.protocol2CodeConverter.asRange(edit.range),
                                edit.newText,
                            ),
                        );
                    }
                }

                snippetTextDocumentEdits.push([uri, snippetTextEdits]);
            }
        }
        return [result, snippetTextDocumentEdits];
    } else {
        // we don't handle WorkspaceEdit.changes since it's not relevant for code actions
        return [result, []];
    }
}

export function applySnippetWorkspaceEditCommand(_ctx: CtxInit): Cmd {
    return async (edit: vscode.WorkspaceEdit) => {
        await applySnippetWorkspaceEdit(edit, edit.entries());
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
