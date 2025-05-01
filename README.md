# Aptos Move Analyzer 

To install:

```
cargo xtask install --server --client
```

It will build `aptos-analyzer.vsix` extension file and install it into your VSCode.
Then it will run `cargo install` for the language server.

Put

```
"aptos-analyzer.server.path": "~/.cargo/bin/aptos-analyzer",
```

to your `settings.json` to point the extension to your locally built language server.

Now, open any Move file to instantiate the extension. Disable other VSCode extensions for `.move` files if needed.

## Features

### Language support

* syntax / semantic highlighting
* go-to-definition
* completion
* lints and quickfixes
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

### Flycheck

Checks code in the editor after saving the document.

To enable, specify in your `settings.json`: 
```json5
{
    "aptos-analyzer.checkOnSave": true,
    "aptos-analyzer.aptosPath": "/home/mkurnikov/bin/aptos", // path to aptos-cli on your machine
}
```

## Debugging

It's useful to enable INFO logging level, it's not very chatty and could provide with a valuable information to debug:

```
    "aptos-analyzer.server.extraEnv": { "RA_LOG": "info" },
```

### Resolve definitions

If there's any issue with missing go-to-definition, the "unresolved reference" diagnostic could be helpful:

```
    "aptos-analyzer.diagnostics.enableUnresolvedReference": true
```

It's disabled by default, as the underlying compiler frontend still incomplete. 



