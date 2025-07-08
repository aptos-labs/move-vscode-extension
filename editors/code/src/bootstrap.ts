// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

import vscode from "vscode";
import { Config } from "./config";
import os from "os";
import { Env, log, spawnAsync } from "./util";

export async function bootstrap(
    context: vscode.ExtensionContext,
    config: Readonly<Config>,
): Promise<string> {
    const path = await getServer(context, config);
    if (!path) {
        throw new Error(
            "Aptos Language Server is not available. " +
            "See README for the [proper installation procedure](https://github.com/aptos-labs/move-vscode-extension/blob/main/README.md).",
        );
    }

    log.info("Using server binary at", path);

    if (!await isValidExecutable(path, config.serverExtraEnv)) {
        throw new Error(
            `Failed to execute ${path} --version.` +
            (config.languageServerPath
                ? `\`config.server.path\` or \`config.serverPath\` has been set explicitly.\
            Consider removing this config or making a valid server binary available at that path.`
                : ""),
        );
    }

    return path;
}

async function getServer(
    context: vscode.ExtensionContext,
    config: Readonly<Config>,
): Promise<string | undefined> {
    const packageJson: {
        version: string;
        releaseTag: string | null;
    } = context.extension.packageJSON;

    // check if the server path is configured explicitly
    const explicitPath = /*process.env["__RA_LSP_SERVER_DEBUG"] ?? */config.languageServerPath;
    if (explicitPath) {
        if (explicitPath.startsWith("~/")) {
            return os.homedir() + explicitPath.slice("~".length);
        }
        return explicitPath;
    }

    // if there's no releaseTag, then it runs the `aptos-language-server` from $PATH
    if (packageJson.releaseTag === null) return "aptos-language-server";

    // finally, use the bundled one
    const ext = process.platform === "win32" ? ".exe" : "";
    const bundled = vscode.Uri.joinPath(context.extensionUri, "server", `aptos-language-server${ext}`);
    const bundledExists = await fileExists(bundled);
    if (bundledExists) {
        return bundled.fsPath;
    }

    await vscode.window.showErrorMessage(
        "Unfortunately we don't ship binaries for your platform yet. " +
        "You need to manually clone the https://github.com/aptos-labs/move-vscode-extension/ repository and " +
        "run `cargo xtask install --server` to build the language server from sources."
    );
    return undefined;
}

async function fileExists(uri: vscode.Uri) {
    return await vscode.workspace.fs.stat(uri).then(
        () => true,
        () => false,
    );
}


export async function isValidExecutable(path: string, extraEnv: Env): Promise<boolean> {
    log.debug("Checking availability of a binary at", path);

    const res = await spawnAsync(path, ["--version"], {
        env: { ...process.env, ...extraEnv },
    });

    if (res.error) {
        log.warn(path, "--version:", res);
    } else {
        log.info(path, "--version:", res);
    }
    return res.status === 0;
}
