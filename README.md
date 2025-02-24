# Aptos Move Analyzer

To install:

```
cargo xtask install --server --client
```

It will build `aptos-analyzer.vsix` extension file and install it into your VSCode.
Then it will run `cargo install` for the language server. 

Open any project with the `Move.toml` in the root. Disable other VSCode extensions for `.move` files if needed.  
