# Move on Aptos Language Extension

This is the official Visual Studio Code (and Cursor) extension for [developing smart contracts in the Move language on the Aptos blockchain](https://aptos.dev/en/build/smart-contracts).

Built from the ground up, it delivers a modern and performant development experience, offering essential features like semantic highlighting, real-time diagnostics, auto-formatting and seamless integration with the rest of the Aptos toolchain — all designed to help developers build and test Move contracts with ease and confidence.

Actively maintained by the Aptos team, this extension is designed to evolve alongside the Move language and supports both developers who are new to Move, and those building more complex applications.

## Features

- Semantic Highlighting  
- Go to Definition  
- Find All References & Symbol Renaming  
- Type and Documentation on Hover
- Inlay Hints for Types  
- Real-Time Diagnostics  
- Quick Fixes (Code Actions)  
- `movefmt` Integration  

## Installation

We publish nightly pre-compiled VSCode extensions for Linux, MacOS (x86 and arm) and Windows. 
Download a `.vsix` bundle for your platform from https://github.com/aptos-labs/move-vscode-extension/releases/tag/nightly, 
then install it in your editor with 
`"Install from VSIX..."`([reference](https://code.visualstudio.com/docs/configure/extensions/extension-marketplace#_install-from-a-vsix)) command.

### Build from sources

Clone the repo, then run:
```
cargo run -p xtask -- install --server --client
```
(or just `cargo xtask install --server --client`, see https://github.com/matklad/cargo-xtask) 

The command builds `move-on-aptos.vsix` extension file and installs it into your VSCode. 
Then it runs `cargo install` to build and install language server.

Put

```
"move-on-aptos.server.path": "~/.cargo/bin/aptos-language-server",
```

to your `settings.json` to point the extension to your locally built language server.

Now, open any Move file to instantiate the extension. Disable other VSCode extensions for `.move` files if needed.

### Cursor AI editor

If you use https://www.cursor.com/ AI editor, you need to do a bit more work.

Run the installation command above. The result would be a `./editors/code/move-on-aptos.vsix` vscode extension package.
Then install it from the editor using the `"Install from VSIX..."` command.

## Recommended configuration for the Move package directories

LSP is somewhat limited in what it can actually do, so some of the settings need to be specified manually. 

### Mark Move Library sources read-only

Add the following to your `settings.json`:

```json5
    "files.readonlyInclude": {
        "**/build/*/sources/**/*.move": true,
        "**/.move/**/*.move": true,
    }
```

### Auto-close `b"` and `x"` properly

```json5
    "[move]": {
        "editor.wordSeparators": "`~!@#$%^&*()-=+[{]}\\|;:'\",.<>/?bx",
    },
```

A bunch of symbols in the config value are the defaults, we're adding `b` and `x` symbols for the string prefixes. 

## Configuration

### Inlay hints

Type hints for the let statements and lambda parameters are supported. 
```move
module 0x1::m {
    fun main() {
        let a/*: integer*/ = 1;
        let f: |u8| u8 = |e/*: u8*/| e;
    }
}
```

To disable those, use:

```json5
{
    "move-on-aptos.inlayHints.typeHints.enable": false,
}
```

### Formatting (works with `movefmt` >= 1.2.1)

Specify a path to the `movefmt` executable and extra args (like a `--config-path`) if necessary:
```json5
{
    "move-on-aptos.movefmt.path": "~/code/movefmt/target/release/movefmt",
    "move-on-aptos.movefmt.extraArgs": [],
}
```

Formatting on Save can be enabled in VSCode with 
```json5
{
    "editor.formatOnSave": true,
}
```

## Debugging

It's useful to enable INFO logging level, it's not very chatty and could provide with a valuable information to debug:

```
    "move-on-aptos.server.extraEnv": { "RA_LOG": "info" },
```

## Additional commands

### `aptos-language-server diagnostics --fix`

Run server diagnostics on the file (or package directory). If `--fix` is provided, automatically applies available autofixes:   

```shell
  $ aptos-language-server diagnostics --fix ./aptos-stdlib/sources/cryptography/keyless.move 
processing package 'aptos-stdlib', file: /home/mkurnikov/code/aptos-core/aptos-move/framework/aptos-stdlib/sources/cryptography/keyless.move
note[replace-with-method-call]: Can be replaced with method call
   ┌─ /home/mkurnikov/code/aptos-core/aptos-move/framework/aptos-stdlib/sources/cryptography/keyless.move:67:17
   │
67 │         assert!(string::bytes(&iss).length() <= MAX_ISSUER_UTF8_BYTES_LENGTH, error::invalid_argument(E_INVALID_ISSUER_UTF8_BYTES_LENGTH));
   │                 ^^^^^^^^^^^^^^^^^^^
   │
   ┌─ /home/mkurnikov/code/aptos-core/aptos-move/framework/aptos-stdlib/sources/cryptography/keyless.move:67:17
   │
67 │         assert!(iss.bytes().length() <= MAX_ISSUER_UTF8_BYTES_LENGTH, error::invalid_argument(E_INVALID_ISSUER_UTF8_BYTES_LENGTH));
   │                 ^^^^^^^^^^^ after fix


```

Available diagnostics with fixes:

* change to receiver style function
```move
vector::push_back(v, 1); -> v.push_back(1); 
```

* change to compound assignment
```move
a = a + 1; -> a += 1;
```

## Roadmap

The end goal is to be at a feature parity with the Intellij-Move plugin. 

Next features planned are (roughly in the expected order of implementation):

* More error highlighting: 
  - Not enough type params / missing fields.
  - Support `aptos move lint` lints with quickfixes.
* Add "item is private" clarification to the "unresolved reference" diagnostic.
* Unused imports (with quickfix).
* Global auto-completion (auto-import).

## Contributing
We welcome feedback, bug reports, and contributions from the community!

If you run into a bug, usability issue, or have a feature request, please don’t hesitate to [open an issue](../../issues). This will help us improve the experience for everyone.

That said, this project is still in its early stages, and many parts of it are evolving quickly. If you're planning to work on a larger change or feature, we encourage you to start a discussion or open an issue first. This helps ensure alignment and avoid unnecessary rework.

## Acknowledgements
This project is inspired by [rust-analyzer](https://github.com/rust-lang/rust-analyzer).

Portions of the code in this project are derived from rust-analyzer and are used under 
the terms of the Apache License, Version 2.0.

We thank the rust-analyzer contributors for their work and inspiration.