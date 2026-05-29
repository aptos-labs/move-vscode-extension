// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import * as vscode from "vscode";
import * as fs from "fs";
import * as path from "path";
import * as zlib from "zlib";
import { log } from "./util";

const OWNER = "aptos-labs";
const REPO = "aptos-debugger";
const UPDATE_CHECK_INTERVAL_MS = 60 * 60 * 1000;

function getAssetName(): string | undefined {
    const platform = process.platform;
    const arch = process.arch;

    if (platform === "darwin" && arch === "arm64") return "aptos-dap-darwin-arm64.gz";
    if (platform === "darwin" && arch === "x64") return "aptos-dap-darwin-x64.gz";
    if (platform === "linux" && arch === "x64") return "aptos-dap-linux-x64.gz";
    return undefined;
}

function storagePaths(context: vscode.ExtensionContext) {
    const dir = context.globalStorageUri.fsPath;
    return {
        binary: path.join(dir, "aptos-dap"),
        version: path.join(dir, "aptos-dap.version"),
        lastCheck: path.join(dir, "aptos-dap.last-check"),
    };
}

async function downloadDap(
    context: vscode.ExtensionContext,
    assetName: string,
    binaryPath: string,
    versionPath: string,
): Promise<string | undefined> {
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
        fs.writeFileSync(versionPath, release.data.tag_name);
        log.info("aptos-dap downloaded to", binaryPath, "version", release.data.tag_name);
        return binaryPath;
    } catch (e: any) {
        void vscode.window.showErrorMessage(`Failed to download aptos-dap: ${e.message}`);
        log.error("aptos-dap download failed", e);
        return undefined;
    }
}

async function checkForUpdate(
    context: vscode.ExtensionContext,
    assetName: string,
    binaryPath: string,
    versionPath: string,
    lastCheckPath: string,
): Promise<void> {
    if (fs.existsSync(lastCheckPath)) {
        const lastCheck = parseInt(fs.readFileSync(lastCheckPath, "utf8").trim(), 10);
        if (Date.now() - lastCheck < UPDATE_CHECK_INTERVAL_MS) return;
    }
    fs.writeFileSync(lastCheckPath, String(Date.now()));

    try {
        const localVersion = fs.existsSync(versionPath)
            ? fs.readFileSync(versionPath, "utf8").trim()
            : undefined;

        const { Octokit } = await import("@octokit/rest");
        const octokit = new Octokit();
        const release = await octokit.repos.getLatestRelease({ owner: OWNER, repo: REPO });
        const latestVersion = release.data.tag_name;

        if (localVersion === latestVersion) return;

        const fromVersion = localVersion ? ` from ${localVersion}` : "";
        const choice = await vscode.window.showInformationMessage(
            `A new aptos-dap release is available: ${latestVersion} (current${fromVersion}). Update?`,
            "Update",
            "Skip",
        );
        if (choice !== "Update") return;

        if (fs.existsSync(binaryPath)) fs.unlinkSync(binaryPath);
        await downloadDap(context, assetName, binaryPath, versionPath);
    } catch (e: any) {
        log.warn("aptos-dap update check failed", e);
    }
}

export async function ensureAptosDapUpToDate(context: vscode.ExtensionContext): Promise<string | undefined> {
    const assetName = getAssetName();
    if (!assetName) {
        void vscode.window.showErrorMessage(
            "aptos-dap is not available for your platform. " +
            "Only macOS (arm64/x64) and Linux (x64) are supported.",
        );
        return undefined;
    }

    const { binary: binaryPath, version: versionPath, lastCheck: lastCheckPath } = storagePaths(context);

    if (fs.existsSync(binaryPath)) {
        void checkForUpdate(context, assetName, binaryPath, versionPath, lastCheckPath);
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

    return downloadDap(context, assetName, binaryPath, versionPath);
}