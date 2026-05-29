// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import * as vscode from "vscode";
import * as fs from "fs";
import * as path from "path";
import * as zlib from "zlib";
import { log } from "./util";

const OWNER = "aptos-labs";
const REPO = "aptos-debugger";

function getAssetName(): string | undefined {
    const platform = process.platform;
    const arch = process.arch;

    if (platform === "darwin" && arch === "arm64") return "aptos-dap-darwin-arm64.gz";
    if (platform === "darwin" && arch === "x64") return "aptos-dap-darwin-x64.gz";
    if (platform === "linux" && arch === "x64") return "aptos-dap-linux-x64.gz";
    return undefined;
}

export async function ensureDapBinary(context: vscode.ExtensionContext): Promise<string | undefined> {
    const assetName = getAssetName();
    if (!assetName) {
        void vscode.window.showErrorMessage(
            "aptos-dap is not available for your platform. " +
            "Only macOS (arm64/x64) and Linux (x64) are supported.",
        );
        return undefined;
    }

    const storageDir = context.globalStorageUri.fsPath;
    const binaryPath = path.join(storageDir, "aptos-dap");

    if (fs.existsSync(binaryPath)) {
        return binaryPath;
    }

    const choice = await vscode.window.showInformationMessage(
        "aptos-dap binary is required for debugging but is not configured. " +
        "Would you like to download it from GitHub?",
        "Download",
        "Cancel",
    );
    if (choice !== "Download") {
        return undefined;
    }

    try {
        const { Octokit } = await import("@octokit/rest");
        const octokit = new Octokit();
        const release = await octokit.repos.getLatestRelease({ owner: OWNER, repo: REPO });
        const asset = release.data.assets.find((a) => a.name === assetName);
        if (!asset) {
            void vscode.window.showErrorMessage(
                `Could not find asset "${assetName}" in the latest aptos-debugger release.`,
            );
            return undefined;
        }

        await vscode.workspace.fs.createDirectory(context.globalStorageUri);

        await vscode.window.withProgress(
            {
                location: vscode.ProgressLocation.Notification,
                title: "Downloading aptos-dap…",
                cancellable: false,
            },
            async () => {
                const response = await octokit.repos.getReleaseAsset({
                    owner: OWNER,
                    repo: REPO,
                    asset_id: asset.id,
                    headers: { accept: "application/octet-stream" },
                });
                const compressed = Buffer.from(response.data as unknown as ArrayBuffer);
                const decompressed = zlib.gunzipSync(compressed);
                fs.writeFileSync(binaryPath, decompressed);
            },
        );

        fs.chmodSync(binaryPath, 0o755);
        log.info("aptos-dap downloaded to", binaryPath);
        return binaryPath;
    } catch (e: any) {
        void vscode.window.showErrorMessage(`Failed to download aptos-dap: ${e.message}`);
        log.error("aptos-dap download failed", e);
        return undefined;
    }
}