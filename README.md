# Aptos Move Analyzer

To install:

```
cargo xtask install --server --client
```

It will build `aptos-analyzer.vsix` extension file and install it into your VSCode.
Then it will run `cargo install` for the language server. 

Open any project with the `Move.toml` in the root. Disable other VSCode extensions for `.move` files if needed.  

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
