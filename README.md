# Aptos Move Analyzer 

## Installation

Clone the repo, then run:
```
cargo xtask install --server --client
```
(it uses https://github.com/matklad/cargo-xtask which is not a separate command, but a cargo aliasing technique, 
and code for the `cargo xtask install` command resides in `xtask` crate).

It will build `aptos-analyzer.vsix` extension file and install it into your VSCode.
Then it will run `cargo install` for the language server.

Put

```
"aptos-analyzer.server.path": "~/.cargo/bin/aptos-analyzer",
```

to your `settings.json` to point the extension to your locally built language server.

Now, open any Move file to instantiate the extension. Disable other VSCode extensions for `.move` files if needed.

### Cursor AI editor

If you use https://www.cursor.com/ AI editor, you need to do a bit more work. 

Run the installation command above. The result would be a `./editors/code/aptos-analyzer.vsix` vscode extension package. 
Install it from the editor using the `"Install from VSIX..."` command.  

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

### Flycheck

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

### Resolve definitions

If there's any issue with missing go-to-definition, the "unresolved reference" diagnostic could be helpful:

```
    "aptos-analyzer.diagnostics.enableUnresolvedReference": true
```

It's disabled by default, as the underlying compiler frontend still incomplete. 


## Debugging

It's useful to enable INFO logging level, it's not very chatty and could provide with a valuable information to debug:

```
    "aptos-analyzer.server.extraEnv": { "RA_LOG": "info" },
```


