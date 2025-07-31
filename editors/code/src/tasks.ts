// This ends up as the `type` key in tasks.json.
import * as vscode from "vscode";
import { ProcessExecutionOptions } from "vscode";

export const APTOS_TASK_TYPE = "aptos";
export const APTOS_TASK_SOURCE = "move";

export type AptosTaskDefinition = vscode.TaskDefinition & {
    type: typeof APTOS_TASK_TYPE;
    subcommand: string;
    args?: string[];
    env?: Record<string, string>;
};

export async function buildAptosTask(
    scope: vscode.WorkspaceFolder | vscode.TaskScope | undefined,
    definition: AptosTaskDefinition,
    name: string,
    exec: vscode.ProcessExecution | vscode.ShellExecution,
): Promise<vscode.Task> {
    return new vscode.Task(
        definition,
        // scope can sometimes be undefined. in these situations we default to the workspace taskscope as
        // recommended by the official docs: https://code.visualstudio.com/api/extension-guides/task-provider#task-provider)
        scope ?? vscode.TaskScope.Workspace,
        name,
        APTOS_TASK_SOURCE,
        exec,
        "$aptos",
    );
}








