import {strict as nativeAssert} from "assert";
import vscode from "vscode";
import {inspect} from "util";

export function assert(condition: boolean, explanation: string): asserts condition {
    try {
        nativeAssert(condition, explanation);
    } catch (err) {
        log.error(`Assertion failed:`, explanation);
        throw err;
    }
}

export type Env = {
    [name: string]: string;
};

class Log {
    private readonly output = vscode.window.createOutputChannel("aptos-analyzer Extension", {
        log: true,
    });

    trace(...messages: [unknown, ...unknown[]]): void {
        this.output.trace(this.stringify(messages));
    }

    debug(...messages: [unknown, ...unknown[]]): void {
        this.output.debug(this.stringify(messages));
    }

    info(...messages: [unknown, ...unknown[]]): void {
        this.output.info(this.stringify(messages));
    }

    warn(...messages: [unknown, ...unknown[]]): void {
        this.output.warn(this.stringify(messages));
    }

    error(...messages: [unknown, ...unknown[]]): void {
        this.output.error(this.stringify(messages));
        this.output.show(true);
    }

    private stringify(messages: unknown[]): string {
        return messages
            .map((message) => {
                if (typeof message === "string") {
                    return message;
                }
                if (message instanceof Error) {
                    return message.stack || message.message;
                }
                return inspect(message, { depth: 6, colors: false });
            })
            .join(" ");
    }
}

export const log = new Log();

export function sleep(ms: number) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}

export type AptosDocument = vscode.TextDocument & { languageId: "move" };
export type AptosEditor = vscode.TextEditor & { document: AptosDocument };

export function isAptosDocument(document: vscode.TextDocument): document is AptosDocument {
    // Prevent corrupted text (particularly via inlay hints) in diff views
    // by allowing only `file` schemes
    // unfortunately extensions that use diff views not always set this
    // to something different than 'file' (see ongoing bug: #4608)
    return document.languageId === "move" && document.uri.scheme === "file";
}

export function isMoveTomlDocument(document: vscode.TextDocument): document is AptosDocument {
    // ideally `document.languageId` should be 'toml' but user maybe not have toml extension installed
    return document.uri.scheme === "file" && document.fileName.endsWith("Move.toml");
}

export function isAptosEditor(editor: vscode.TextEditor): editor is AptosEditor {
    return isAptosDocument(editor.document);
}

/** Sets ['when'](https://code.visualstudio.com/docs/getstarted/keybindings#_when-clause-contexts) clause contexts */
export function setContextValue(key: string, value: any): Thenable<void> {
    return vscode.commands.executeCommand("setContext", key, value);
}

export class LazyOutputChannel implements vscode.OutputChannel {
    constructor(name: string) {
        this.name = name;
    }

    name: string;
    _channel: vscode.OutputChannel | undefined;

    get channel(): vscode.OutputChannel {
        if (!this._channel) {
            this._channel = vscode.window.createOutputChannel(this.name);
        }
        return this._channel;
    }

    append(value: string): void {
        this.channel.append(value);
    }
    appendLine(value: string): void {
        this.channel.appendLine(value);
    }
    replace(value: string): void {
        this.channel.replace(value);
    }
    clear(): void {
        if (this._channel) {
            this._channel.clear();
        }
    }
    show(preserveFocus?: boolean): void;
    show(column?: vscode.ViewColumn, preserveFocus?: boolean): void;
    show(column?: any, preserveFocus?: any): void {
        this.channel.show(column, preserveFocus);
    }
    hide(): void {
        if (this._channel) {
            this._channel.hide();
        }
    }
    dispose(): void {
        if (this._channel) {
            this._channel.dispose();
        }
    }
}


export type NotUndefined<T> = T extends undefined ? never : T;

export type Undefinable<T> = T | undefined;

function isNotUndefined<T>(input: Undefinable<T>): input is NotUndefined<T> {
    return input !== undefined;
}

export function expectNotUndefined<T>(input: Undefinable<T>, msg: string): NotUndefined<T> {
    if (isNotUndefined(input)) {
        return input;
    }

    throw new TypeError(msg);
}

export function unwrapUndefinable<T>(input: Undefinable<T>): NotUndefined<T> {
    return expectNotUndefined(input, `unwrapping \`undefined\``);
}
