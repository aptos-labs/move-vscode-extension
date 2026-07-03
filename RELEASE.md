# Release process

To publish a release, just push to the `release` branch of the repository.   

```bash
git clone git@github.com:aptos-labs/move-vscode-extension.git
cd move-vscode-extension
git checkout release
git rebase main
git push
```

CI will create a new release with `YYYY-MM-DD` name, build and upload release assets, and publish extensions 
both to the VSCode Marketplace and to the OpenVSX Registry. 
Under the hood, it uses `cargo xtask dist` command, see `./xtask/src/dist.rs`. 

Pushing to `release` twice on the same day will reuse the same GitHub release tag. 
The release assets are overwritten, and the extensions are published again with a new CI run-number patch version.

See `.github/workflows/release.yml` for the CI configuration. 
It loosely follows rust-analyzer's release process, 
with some simplifications (https://github.com/rust-lang/rust-analyzer/blob/master/.github/workflows/release.yaml). 

## Publishing tokens

The publishing tokens are saved as secrets in this repo, 
`MARKETPLACE_PAT_TOKEN` (for VSCode publishing API token) and `OPENVSX_TOKEN` (for OpenVSX). 

See
https://code.visualstudio.com/api/working-with-extensions/publishing-extension#get-a-personal-access-token
https://github.com/eclipse-openvsx/openvsx/wiki/Publishing-Extensions
for more.