# Aptos Move Analyzer 

## Installation

We publish nightly pre-compiled VSCode extensions for Linux, MacOS and Windows. Check out https://github.com/aptos-labs/aptos-move-analyzer/releases/tag/nightly. 

Download the `.vsix` file for your platform, then install it inside your editor with `"Install from VSIX..."` command. 

### Build from sources

Clone the repo, then run:
```
cargo run -p xtask -- install --server --client
```
(or just `cargo xtask install --server --client`, see https://github.com/matklad/cargo-xtask) 

The command builds `aptos-analyzer.vsix` extension file and installs it into your VSCode. 
Then it runs `cargo install` to build and install language server.

Put

```
"aptos-analyzer.server.path": "~/.cargo/bin/aptos-analyzer",
```

to your `settings.json` to point the extension to your locally built language server.

Now, open any Move file to instantiate the extension. Disable other VSCode extensions for `.move` files if needed.

### VSCode configuration

Add the following to settings.json to mark Move library sources as read-only:

```json5
    "files.readonlyInclude": {
        "**/build/*/sources/**/*.move": true,
        "**/.move/**/*.move": true,
    }
```

### Cursor AI editor

If you use https://www.cursor.com/ AI editor, you need to do a bit more work. 

Run the installation command above. The result would be a `./editors/code/aptos-analyzer.vsix` vscode extension package. 
Then install it from the editor using the `"Install from VSIX..."` command.  

## Features

### Language support

* syntax / semantic highlighting
* go-to-definition
* completion
* lints and quickfixes
* inlay type hints
```
module 0x1::m {
    struct S { val: u8 }
    fun method(self: S, a: u8, b: u8): u8 {
        self.val
    }
    fun main(s: S) {
        method(s, 1, 2);
      //^^^^^^^^^^^^^^^ weak: Can be replaced with method call
    }
}
  ```

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
    "aptos-analyzer.inlayHints.typeHints.enable": false,
}
```

### Diagnostics

If there's any issue with missing go-to-definition, the "unresolved reference" diagnostic could be helpful:

```
    "aptos-analyzer.diagnostics.enableUnresolvedReference": true
```

Same for type checking:

```
    "aptos-analyzer.diagnostics.enableTypeChecking": true
```

### Formatting (for `movefmt` >= 1.2.1)

Specify a path to the `movefmt` executable and extra args (like a `--config-path`) if necessary:
```json5
{
    "aptos-analyzer.movefmt.path": "~/code/movefmt/target/release/movefmt",
    "aptos-analyzer.movefmt.extraArgs": [],
}
```

Formatting on Save can be enabled in VSCode with 
```json5
{
    "editor.formatOnSave": true,
}
```

### Aptos Compiler check on Save

Checks code in the editor after saving the document by running `aptos move compile`.

To enable, specify in your `settings.json`:
```json5
{
    "aptos-analyzer.checkOnSave": true,
    "aptos-analyzer.aptosPath": "/home/mkurnikov/bin/aptos", // path to aptos-cli on your machine
}
```

To provide additional arguments to the `compile` command, use `aptos-analyzer.check.extraArgs`:

```json5
{   
    "aptos-analyzer.check.extraArgs": ["--dev"],
}
```

To run `aptos move lint` instead, specify custom `aptos move` command with:
```json5
{
    "aptos-analyzer.check.command": "lint",
}
```

## Debugging

It's useful to enable INFO logging level, it's not very chatty and could provide with a valuable information to debug:

```
    "aptos-analyzer.server.extraEnv": { "RA_LOG": "info" },
```

## Additional commands

### `aptos-analyzer check --fix`

Run server diagnostics on the file (or package directory). If `--fix` is provided, automatically applies available autofixes:   

```shell
  $ aptos-analyzer check --fix ./aptos-stdlib/sources/cryptography/keyless.move 
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
  - Not enough type params / params / missing fields.
  - Replace `vector::borrow` with `v[]`.
  - `aptos move lint` lints with quickfixes.
* Find references.
* Unused local variable lint (with quickfix to prefix name with `_`).
* Rename refactoring.
* Add "item is private" clarification to the "unresolved reference" diagnostic.
* Unused imports (with quickfix).
* Auto-import.
