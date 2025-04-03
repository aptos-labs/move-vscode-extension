// The module 'vscode' contains the VS Code extensibility API
// Import the module and reference it with the alias vscode in your code below
import * as vscode from 'vscode';
import {CommandFactory, Ctx} from './ctx';
import * as commands from "./commands";
import {Configuration} from './config';
import {setContextValue} from "./util";
import commandExists from "command-exists";

const APTOS_PROJECT_CONTEXT_NAME = "inAptosProject";

// This method is called when your extension is deactivated
export async function deactivate() {
    await setContextValue(APTOS_PROJECT_CONTEXT_NAME, undefined);
}

// This method is called when your extension is activated
// Your extension is activated the very first time the command is executed
export async function activate(extensionContext: Readonly<vscode.ExtensionContext>) {

    const configuration = new Configuration();

    const serverPath = configuration.serverPath;
    if (!commandExists.sync(serverPath)) {
        const context = new Error(
            `language server executable '${serverPath}' could not be found, so ` +
            'most extension features will be unavailable to you. Follow the instructions in ' +
            'the aptos-analyzer Visual Studio Code extension README to install the language ' +
            'server.',
        );
        // An error here -- for example, if the path to the `aptos-analyzer` binary that the user
        // specified in their settings is not valid -- prevents the extension from providing any
        // more utility, so return early.
        void vscode.window.showErrorMessage(
            `Could not activate aptos-analyzer: ${context.message}.`,
        );
        return;
    }

    const context = new Ctx(extensionContext, configuration, createCommands())
    context.configureLanguage();

    await context.start();

    await setContextValue(APTOS_PROJECT_CONTEXT_NAME, true);
}

function createCommands(): Record<string, CommandFactory> {
    return {
        // onEnter: {
        // 	enabled: commands.onEnter,
        // 	disabled: (_) => () => vscode.commands.executeCommand("default:type", { text: "\n" }),
        // },
        // restartServer: {
        // 	enabled: (ctx) => async () => {
        // 		await ctx.restart();
        // 	},
        // 	disabled: (ctx) => async () => {
        // 		await ctx.start();
        // 	},
        // },
        // startServer: {
        // 	enabled: (ctx) => async () => {
        // 		await ctx.start();
        // 	},
        // 	disabled: (ctx) => async () => {
        // 		await ctx.start();
        // 	},
        // },
        // stopServer: {
        // 	enabled: (ctx) => async () => {
        // 		// FIXME: We should re-use the client, that is ctx.deactivate() if none of the configs have changed
        // 		await ctx.stopAndDispose();
        // 		ctx.setServerStatus({
        // 			health: "stopped",
        // 		});
        // 	},
        // 	disabled: (_) => async () => {},
        // },

        // analyzerStatus: { enabled: commands.analyzerStatus },
        // memoryUsage: { enabled: commands.memoryUsage },
        // reloadWorkspace: { enabled: commands.reloadWorkspace },
        // rebuildProcMacros: { enabled: commands.rebuildProcMacros },
        // matchingBrace: { enabled: commands.matchingBrace },
        // joinLines: { enabled: commands.joinLines },
        // parentModule: { enabled: commands.parentModule },
        // viewHir: { enabled: commands.viewHir },
        // viewMir: { enabled: commands.viewMir },
        // interpretFunction: { enabled: commands.interpretFunction },
        // viewFileText: { enabled: commands.viewFileText },
        // viewItemTree: { enabled: commands.viewItemTree },
        // viewCrateGraph: { enabled: commands.viewCrateGraph },
        // viewFullCrateGraph: { enabled: commands.viewFullCrateGraph },
        // expandMacro: { enabled: commands.expandMacro },
        // run: { enabled: commands.run },
        // copyRunCommandLine: { enabled: commands.copyRunCommandLine },
        // debug: { enabled: commands.debug },
        // newDebugConfig: { enabled: commands.newDebugConfig },
        // openDocs: { enabled: commands.openDocs },
        // openExternalDocs: { enabled: commands.openExternalDocs },
        // openCargoToml: { enabled: commands.openCargoToml },
        // peekTests: { enabled: commands.peekTests },
        // moveItemUp: { enabled: commands.moveItemUp },
        // moveItemDown: { enabled: commands.moveItemDown },
        // cancelFlycheck: { enabled: commands.cancelFlycheck },
        // clearFlycheck: { enabled: commands.clearFlycheck },
        // runFlycheck: { enabled: commands.runFlycheck },
        // ssr: { enabled: commands.ssr },
        // serverVersion: { enabled: commands.serverVersion },
        // viewMemoryLayout: { enabled: commands.viewMemoryLayout },
        // toggleCheckOnSave: { enabled: commands.toggleCheckOnSave },
        toggleLSPLogs: {enabled: commands.toggleLSPLogs},
        // openWalkthrough: { enabled: commands.openWalkthrough },
        // // Internal commands which are invoked by the server.
        // applyActionGroup: { enabled: commands.applyActionGroup },
        // applySnippetWorkspaceEdit: { enabled: commands.applySnippetWorkspaceEditCommand },
        // debugSingle: { enabled: commands.debugSingle },
        // gotoLocation: { enabled: commands.gotoLocation },
        // hoverRefCommandProxy: { enabled: commands.hoverRefCommandProxy },
        // resolveCodeAction: { enabled: commands.resolveCodeAction },
        // runSingle: { enabled: commands.runSingle },
        // showReferences: { enabled: commands.showReferences },
        // triggerParameterHints: { enabled: commands.triggerParameterHints },
        // rename: { enabled: commands.rename },
        // openLogs: { enabled: commands.openLogs },
        // revealDependency: { enabled: commands.revealDependency },
        // syntaxTreeReveal: { enabled: commands.syntaxTreeReveal },
        // syntaxTreeCopy: { enabled: commands.syntaxTreeCopy },
        syntaxTreeHideWhitespace: { enabled: commands.syntaxTreeHideWhitespace },
        syntaxTreeShowWhitespace: { enabled: commands.syntaxTreeShowWhitespace },
    };
}

