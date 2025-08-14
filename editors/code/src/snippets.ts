import vscode from "vscode";

export async function applyTextEdits(editor: vscode.TextEditor, edits: vscode.TextEdit[]) {
    const edit = new vscode.WorkspaceEdit();
    edit.set(editor.document.uri, edits);
    await vscode.workspace.applyEdit(edit);
}
