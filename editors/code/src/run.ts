import * as vscode from "vscode";
import * as lsp_ext from "./lsp_ext";
import * as tasks from "./tasks";
import { unwrapUndefinable } from "./util";

export function prepareEnv(): Record<string, string> {
    const env: Record<string, string> = { RUST_BACKTRACE: "short" };

    Object.assign(env, process.env as { [key: string]: string });

    return env;
}

export async function createTaskFromRunnable(runnable: lsp_ext.Runnable): Promise<vscode.Task> {
    const target = vscode.workspace.workspaceFolders?.[0];

    const runnableArgs = runnable.args;
    let args = runnableArgs.args;

    const definition: tasks.AptosTaskDefinition = {
        type: tasks.APTOS_TASK_TYPE,
        subcommand: unwrapUndefinable(args[0]),
        args: args.slice(1),
    };
    const options = {
        cwd: runnableArgs.workspaceRoot,
        env: prepareEnv(),
    };

    const exec = await tasks.newProcessExecution(definition, options);
    const task = await tasks.buildAptosTask(
        target,
        definition,
        runnable.label,
        exec,
    );

    task.presentationOptions.clear = true;
    // Sadly, this doesn't prevent focus stealing if the terminal is currently
    // hidden, and will become revealed due to task execution.
    task.presentationOptions.focus = false;

    return task;
}

