import vscode from "vscode";
import {Cmd, Ctx} from "./ctx";

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
