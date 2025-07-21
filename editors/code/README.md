# Move on Aptos Language Extension

This is the official Visual Studio Code (and Cursor) extension
for [developing smart contracts in the Move language on the Aptos blockchain](https://aptos.dev/en/build/smart-contracts).

It's recommended over and replaces `movebit.aptos-move-analyzer`. 

## Features

- Semantic Highlighting
- Go to Definition
- Find All References & Symbol Renaming
- Type and Documentation on Hover
- Inlay Hints for Types and Function Parameters
- Real-Time Diagnostics
- Code suggestions
- `movefmt` Integration

## Configuration

> Extension by itself won't download your dependencies from the network.
> 
> If you see `unresolved reference` errors on the `AptosFramework` imports - 
> try running `aptos move compile` once on your project to download your remote dependencies.

This extension provides configurations through VSCode's configuration settings.
All configurations are under `move-on-aptos.*`. 

See the [configuration docs](https://github.com/aptos-labs/move-vscode-extension/blob/main/docs/configuration.md) 
for more information.


