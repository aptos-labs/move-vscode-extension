import vscode, { type Disposable, IndentAction } from "vscode";
import * as Is from "vscode-languageclient/lib/common/utils/is";
import os from "os";
import Path from "path";
import { Env, expectNotUndefined, log, unwrapUndefinable } from "./util";
import path from "path";

type ShowStatusBar = "always" | "never" | { documentSelector: vscode.DocumentSelector };

export class Config {
    readonly extensionId = "aptos.aptos-analyzer";
    configureLang: vscode.Disposable | undefined;

    readonly rootSection = "aptos-analyzer";

    constructor(disposables: Disposable[]) {
        vscode.workspace.onDidChangeConfiguration(this.onDidChangeConfiguration, this, disposables);
        this.refreshLogging();
        this.configureLanguage();
    }

    dispose() {
        this.configureLang?.dispose();
    }

    private refreshLogging() {
        log.info(
            "Extension version:",
            vscode.extensions.getExtension(this.extensionId)!.packageJSON.version,
        );

        const cfg = Object.entries(this.cfg).filter(([_, val]) => !(val instanceof Function));
        log.info("Using configuration", Object.fromEntries(cfg));
    }

    private async onDidChangeConfiguration(event: vscode.ConfigurationChangeEvent) {
        this.refreshLogging();

        this.configureLanguage();

        // const requiresWindowReloadOpt = this.requiresWindowReloadOpts.find((opt) =>
        //     event.affectsConfiguration(opt),
        // );
        //
        // if (requiresWindowReloadOpt) {
        //     const message = `Changing "${requiresWindowReloadOpt}" requires a window reload`;
        //     const userResponse = await vscode.window.showInformationMessage(message, "Reload now");
        //
        //     if (userResponse) {
        //         await vscode.commands.executeCommand("workbench.action.reloadWindow");
        //     }
        // }

        // const requiresServerReloadOpt = this.requiresServerReloadOpts.find((opt) =>
        //     event.affectsConfiguration(opt),
        // );

        // if (!requiresServerReloadOpt) return;
        //
        // if (this.restartServerOnConfigChange) {
        //     await vscode.commands.executeCommand("rust-analyzer.restartServer");
        //     return;
        // }

        // const message = `Changing "${requiresServerReloadOpt}" requires a server restart`;
        // const userResponse = await vscode.window.showInformationMessage(message, "Restart now");

        // if (userResponse) {
        //     const command = "aptos-analyzer.restartServer";
        //     await vscode.commands.executeCommand(command);
        // }
    }

    /**
     * Sets up additional language configuration that's impossible to do via a
     * separate language-configuration.json file. See [1] for more information.
     *
     * This code originates from [2](vscode-rust).
     *
     * [1]: https://github.com/Microsoft/vscode/issues/11514#issuecomment-244707076
     * [2]: https://github.com/rust-lang/vscode-rust/blob/660b412701fe2ea62fad180c40ee4f8a60571c61/src/extension.ts#L287:L287
     */
    configureLanguage(): void {
        // Only need to dispose of the config if there's a change
        if (this.configureLang) {
            this.configureLang.dispose();
            this.configureLang = undefined;
        }

        this.configureLang = vscode.languages.setLanguageConfiguration('move', {
            onEnterRules: [
                {
                    // Doc single-line comment
                    // e.g. ///|
                    beforeText: /^\s*\/{3}.*$/,
                    action: { indentAction: IndentAction.None, appendText: '/// ' },
                },
                {
                    // Parent doc single-line comment
                    // e.g. //!|
                    beforeText: /^\s*\/{2}!.*$/,
                    action: { indentAction: IndentAction.None, appendText: '//! ' },
                },
            ],
        });

        // this.extCtx.subscriptions.push(disposable);
    }

    /** The path to the aptos-analyzer executable. */
    get serverPath() {
        let serverPath = this.cfg.get<string>('server.path');
        if (!serverPath) {
            return undefined;
        }

        if (serverPath.startsWith('~/')) {
            serverPath = os.homedir() + serverPath.slice('~'.length);
        }

        if (process.platform === 'win32' && !serverPath.endsWith('.exe')) {
            serverPath = serverPath + '.exe';
        }
        return Path.resolve(serverPath);
    }

    get serverExtraEnv(): Env {
        const extraEnv =
            this.cfg.get<{ [key: string]: string | number } | null>("server.extraEnv") ?? {};
        return substituteVariablesInEnv(
            Object.fromEntries(
                Object.entries(extraEnv).map(([k, v]) => [
                    k,
                    typeof v !== "string" ? v.toString() : v,
                ]),
            ),
        );
    }

    // /** The path to aptos-cli executable. */
    // async aptosPath() {
    //     let aptosPath = this.cfg.get<string>('aptosPath');
    //     if (!aptosPath) {
    //         // try to find it in $PATH
    //         let aptos = await which("aptos", { nothrow: true });
    //         if (aptos === null) {
    //             return undefined;
    //         }
    //         return Path.resolve(aptos);
    //     }
    //
    //     if (aptosPath.startsWith('~/')) {
    //         aptosPath = os.homedir() + aptosPath.slice('~'.length);
    //     }
    //
    //     if (process.platform === 'win32' && !aptosPath.endsWith('.exe')) {
    //         aptosPath = aptosPath + '.exe';
    //     }
    //     return Path.resolve(aptosPath);
    // }

    get showSyntaxTree() {
        return this.get<boolean>("showSyntaxTree");
    }

    get checkOnSave() {
        return this.get<boolean>("checkOnSave") ?? false;
    }


    get statusBarClickAction() {
        return this.get<string>("statusBar.clickAction");
    }

    get statusBarShowStatusBar() {
        return this.get<ShowStatusBar>("statusBar.showStatusBar");
    }

    get initializeStopped() {
        return this.get<boolean>("initializeStopped");
    }

    /**
     * Beware that postfix `!` operator erases both `null` and `undefined`.
     * This is why the following doesn't work as expected:
     *
     * ```ts
     * const nullableNum = vscode
     *  .workspace
     *  .getConfiguration
     *  .getConfiguration("aptos-analyzer")
     *  .get<number | null>(path)!;
     *
     * // What happens is that type of `nullableNum` is `number` but not `null | number`:
     * const fullFledgedNum: number = nullableNum;
     * ```
     * So this getter handles this quirk by not requiring the caller to use postfix `!`
     */
    private get<T>(path: string): T | undefined {
        return prepareVSCodeConfig(this.cfg.get<T>(path));
    }

    private get cfg(): vscode.WorkspaceConfiguration {
        return vscode.workspace.getConfiguration(this.rootSection);
    }
}

export function prepareVSCodeConfig<T>(resp: T): T {
    if (Is.string(resp)) {
        return substituteVSCodeVariableInString(resp) as T;
    } else if (resp && Is.array<any>(resp)) {
        return resp.map((val) => {
            return prepareVSCodeConfig(val);
        }) as T;
    } else if (resp && typeof resp === "object") {
        const res: { [key: string]: any } = {};
        for (const key in resp) {
            const val = resp[key];
            res[key] = prepareVSCodeConfig(val);
        }
        return res as T;
    }
    return resp;
}

// FIXME: Merge this with `substituteVSCodeVariables` above
export function substituteVariablesInEnv(env: Env): Env {
    const missingDeps = new Set<string>();
    // vscode uses `env:ENV_NAME` for env vars resolution, and it's easier
    // to follow the same convention for our dependency tracking
    const definedEnvKeys = new Set(Object.keys(env).map((key) => `env:${key}`));
    const envWithDeps = Object.fromEntries(
        Object.entries(env).map(([key, value]) => {
            const deps = new Set<string>();
            const depRe = new RegExp(/\${(?<depName>.+?)}/g);
            let match = undefined;
            while ((match = depRe.exec(value))) {
                const depName = unwrapUndefinable(match.groups?.["depName"]);
                deps.add(depName);
                // `depName` at this point can have a form of `expression` or
                // `prefix:expression`
                if (!definedEnvKeys.has(depName)) {
                    missingDeps.add(depName);
                }
            }
            return [`env:${key}`, { deps: [...deps], value }];
        }),
    );

    const resolved = new Set<string>();
    for (const dep of missingDeps) {
        const match = /(?<prefix>.*?):(?<body>.+)/.exec(dep);
        if (match) {
            const { prefix, body } = match.groups!;
            if (prefix === "env") {
                const envName = unwrapUndefinable(body);
                envWithDeps[dep] = {
                    value: process.env[envName] ?? "",
                    deps: [],
                };
                resolved.add(dep);
            } else {
                // we can't handle other prefixes at the moment
                // leave values as is, but still mark them as resolved
                envWithDeps[dep] = {
                    value: "${" + dep + "}",
                    deps: [],
                };
                resolved.add(dep);
            }
        } else {
            envWithDeps[dep] = {
                value: computeVscodeVar(dep) || "${" + dep + "}",
                deps: [],
            };
        }
    }
    const toResolve = new Set(Object.keys(envWithDeps));

    let leftToResolveSize;
    do {
        leftToResolveSize = toResolve.size;
        for (const key of toResolve) {
            const item = unwrapUndefinable(envWithDeps[key]);
            if (item.deps.every((dep) => resolved.has(dep))) {
                item.value = item.value.replace(/\${(?<depName>.+?)}/g, (_wholeMatch, depName) => {
                    const item = unwrapUndefinable(envWithDeps[depName]);
                    return item.value;
                });
                resolved.add(key);
                toResolve.delete(key);
            }
        }
    } while (toResolve.size > 0 && toResolve.size < leftToResolveSize);

    const resolvedEnv: Env = {};
    for (const key of Object.keys(env)) {
        const item = unwrapUndefinable(envWithDeps[`env:${key}`]);
        resolvedEnv[key] = item.value;
    }
    return resolvedEnv;
}


const VarRegex = new RegExp(/\$\{(.+?)\}/g);

function substituteVSCodeVariableInString(val: string): string {
    return val.replace(VarRegex, (substring: string, varName) => {
        if (Is.string(varName)) {
            return computeVscodeVar(varName) || substring;
        } else {
            return substring;
        }
    });
}

function computeVscodeVar(varName: string): string | null {
    const workspaceFolder = () => {
        const folders = vscode.workspace.workspaceFolders ?? [];
        const folder = folders[0];
        // TODO: support for remote workspaces?
        const fsPath: string =
            folder === undefined
                ? // no workspace opened
                ""
                : // could use currently opened document to detect the correct
                  // workspace. However, that would be determined by the document
                  // user has opened on Editor startup. Could lead to
                  // unpredictable workspace selection in practice.
                  // It's better to pick the first one
                folder.uri.fsPath;
        return fsPath;
    };
    // https://code.visualstudio.com/docs/editor/variables-reference
    const supportedVariables: { [k: string]: () => string } = {
        workspaceFolder,

        workspaceFolderBasename: () => {
            return path.basename(workspaceFolder());
        },

        cwd: () => process.cwd(),
        userHome: () => os.homedir(),

        // see
        // https://github.com/microsoft/vscode/blob/08ac1bb67ca2459496b272d8f4a908757f24f56f/src/vs/workbench/api/common/extHostVariableResolverService.ts#L81
        // or
        // https://github.com/microsoft/vscode/blob/29eb316bb9f154b7870eb5204ec7f2e7cf649bec/src/vs/server/node/remoteTerminalChannel.ts#L56
        execPath: () => process.env["VSCODE_EXEC_PATH"] ?? process.execPath,

        pathSeparator: () => path.sep,
    };

    if (varName in supportedVariables) {
        const fn = expectNotUndefined(
            supportedVariables[varName],
            `${varName} should not be undefined here`,
        );
        return fn();
    } else {
        // return "${" + varName + "}";
        return null;
    }
}
