// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

import * as lc from "vscode-languageclient/node";
import * as vscode from "vscode";
import { prepareVSCodeConfig } from "./config";

export async function createClient(
    traceOutputChannel: vscode.OutputChannel,
    outputChannel: vscode.OutputChannel,
    serverOptions: lc.ServerOptions,
): Promise<lc.LanguageClient> {

    const lspMiddleware: lc.Middleware = {
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
    };


    const clientOptions: lc.LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'move' }],
        traceOutputChannel,
        outputChannel,
        middleware: lspMiddleware,
        markdown: {
            supportHtml: true,
        },
    };

    const client = new lc.LanguageClient(
        'move-on-aptos',
        'Move-on-Aptos Language Client',
        serverOptions,
        clientOptions,
    );

    // To turn on all proposed features use: client.registerProposedFeatures();
    client.registerFeature(new ExperimentalFeatures(/*config*/));

    return client;
}

class ExperimentalFeatures implements lc.StaticFeature {
    getState(): lc.FeatureState {
        return { kind: "static" };
    }

    fillClientCapabilities(capabilities: lc.ClientCapabilities): void {
        capabilities.experimental = {
            serverStatusNotification: true,
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
