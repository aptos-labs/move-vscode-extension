// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

// The module 'vscode' contains the VS Code extensibility API
// Import the module and reference it with the alias vscode in your code below
import * as vscode from 'vscode';
import * as lc from "vscode-languageclient/node";

import { CommandFactory, Ctx, fetchWorkspace } from './ctx';
import * as commands from "./commands";
import { setContextValue } from "./util";

const APTOS_PROJECT_CONTEXT_NAME = "inAptosProject";

// This method is called when your extension is deactivated
export async function deactivate() {
    await setContextValue(APTOS_PROJECT_CONTEXT_NAME, undefined);
}

// This method is called when your extension is activated
// Your extension is activated the very first time the command is executed
export async function activate(
    context: Readonly<vscode.ExtensionContext>
) {
    checkConflictingExtensions();

    const ctx = new Ctx(context, createCommands(), fetchWorkspace());
    // VS Code doesn't show a notification when an extension fails to activate
    // so we do it ourselves.
    await activateServer(ctx).catch((err) => {
        void vscode.window.showErrorMessage(
            `Cannot activate move-on-aptos extension: ${err.message}`,
        );
        throw err;
    });

    await setContextValue(APTOS_PROJECT_CONTEXT_NAME, true);
}

async function activateServer(ctx: Ctx): Promise<Ctx> {
    // if (ctx.workspace.kind === "Workspace Folder") {
    //     ctx.pushExtCleanup(activateTaskProvider(ctx.config));
    // }

    vscode.workspace.onDidChangeWorkspaceFolders(
        async (_) => ctx.onWorkspaceFolderChanges(),
        null,
        ctx.subscriptions,
    );
    vscode.workspace.onDidChangeConfiguration(
        async (_) => {
            await ctx.client?.sendNotification(lc.DidChangeConfigurationNotification.type, {
                settings: "",
            });
        },
        null,
        ctx.subscriptions,
    );

    if (ctx.config.initializeStopped) {
        ctx.setServerStatus({
            health: "stopped",
        });
    } else {
        await ctx.start();
    }

    return ctx;
}

function createCommands(): Record<string, CommandFactory> {
    return {
        // onEnter: {
        // 	enabled: commands.onEnter,
        // 	disabled: (_) => () => vscode.commands.executeCommand("default:type", { text: "\n" }),
        // },
        restartServer: {
            enabled: (ctx) => async () => {
                await ctx.restart();
            },
            disabled: (ctx) => async () => {
                await ctx.start();
            },
        },
        startServer: {
            enabled: (ctx) => async () => {
                await ctx.start();
            },
            disabled: (ctx) => async () => {
                await ctx.start();
            },
        },
        stopServer: {
            enabled: (ctx) => async () => {
                // FIXME: We should re-use the client, that is ctx.deactivate() if none of the configs have changed
                await ctx.stopAndDispose();
                ctx.setServerStatus({
                    health: "stopped",
                });
            },
            disabled: (_) => async () => {
            },
        },

        analyzerStatus: { enabled: commands.analyzerStatus },
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
        // ssr: { enabled: commands.ssr },
        serverVersion: { enabled: commands.serverVersion },
        // viewMemoryLayout: { enabled: commands.viewMemoryLayout },
        // toggleCheckOnSave: { enabled: commands.toggleCheckOnSave },
        toggleLSPLogs: { enabled: commands.toggleLSPLogs },
        // openWalkthrough: { enabled: commands.openWalkthrough },
        // // Internal commands which are invoked by the server.
        // debugSingle: { enabled: commands.debugSingle },
        // gotoLocation: { enabled: commands.gotoLocation },
        // hoverRefCommandProxy: { enabled: commands.hoverRefCommandProxy },
        // runSingle: { enabled: commands.runSingle },
        // showReferences: { enabled: commands.showReferences },
        // triggerParameterHints: { enabled: commands.triggerParameterHints },
        // rename: { enabled: commands.rename },
        openLogs: { enabled: commands.openLogs },
        // runAptosUpdateMovefmt: { enabled: commands.runAptosUpdateMovefmt },
        // syntaxTreeReveal: { enabled: commands.syntaxTreeReveal },
        // syntaxTreeCopy: { enabled: commands.syntaxTreeCopy },
        syntaxTreeHideWhitespace: { enabled: commands.syntaxTreeHideWhitespace },
        syntaxTreeShowWhitespace: { enabled: commands.syntaxTreeShowWhitespace },
    };
}

function checkConflictingExtensions() {
    if (vscode.extensions.getExtension("MoveBit.aptos-move-analyzer")) {
        vscode.window
            .showWarningMessage(
                `You have both the move-on-aptos (aptoslabs.move-on-aptos) and MoveBit's aptos-move-analyzer (MoveBit.aptos-move-analyzer) ` +
                "plugins enabled. These are known to conflict and cause various functions of " +
                "both plugins to not work correctly. You should disable one of them.",
                "Got it",
            )
            // eslint-disable-next-line no-console
            .then(() => {
            }, console.error);
    }
}

