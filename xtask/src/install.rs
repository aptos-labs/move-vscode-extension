use anyhow::{bail, Context};
use xshell::{cmd, Shell};

pub(crate) fn install_client(sh: &Shell) -> anyhow::Result<()> {
    let _dir = sh.push_dir("./editors/code");

    // Package extension.
    if cfg!(unix) {
        cmd!(sh, "npm --version")
            .run()
            .context("`npm` is required to build the VS Code plugin")?;
        cmd!(sh, "npm ci").run()?;

        cmd!(sh, "npm run package --scripts-prepend-node-path").run()?;
    } else {
        cmd!(sh, "cmd.exe /c npm --version")
            .run()
            .context("`npm` is required to build the VS Code plugin")?;
        cmd!(sh, "cmd.exe /c npm ci").run()?;

        cmd!(sh, "cmd.exe /c npm run package").run()?;
    };

    let code = "code";
    //
    // // Find the appropriate VS Code binary.
    // let lifetime_extender;
    // let candidates: &[&str] = match client_opt.code_bin.as_deref() {
    //     Some(it) => {
    //         lifetime_extender = [it];
    //         &lifetime_extender[..]
    //     }
    //     None => VS_CODES,
    // };
    // let code = candidates
    //     .iter()
    //     .copied()
    //     .find(|&bin| {
    //         if cfg!(unix) {
    //             cmd!(sh, "{bin} --version").read().is_ok()
    //         } else {
    //             cmd!(sh, "cmd.exe /c {bin}.cmd --version").read().is_ok()
    //         }
    //     })
    //     .ok_or_else(|| {
    //         format_err!("Can't execute `{} --version`. Perhaps it is not in $PATH?", candidates[0])
    //     })?;

    // Install & verify.
    let installed_extensions = if cfg!(unix) {
        cmd!(sh, "{code} --install-extension aptos-analyzer.vsix --force").run()?;
        cmd!(sh, "{code} --list-extensions").read()?
    } else {
        cmd!(
            sh,
            "cmd.exe /c {code}.cmd --install-extension aptos-analyzer.vsix --force"
        )
        .run()?;
        cmd!(sh, "cmd.exe /c {code}.cmd --list-extensions").read()?
    };

    if !installed_extensions.contains("aptos-analyzer") {
        bail!(
            "Could not install the Visual Studio Code extension. \
            Please make sure you have at least NodeJS 16.x together with the latest version of VS Code installed and try again. \
            Note that installing via xtask install does not work for VS Code Remote, instead you’ll need to install the .vsix manually."
        );
    }

    Ok(())
}

pub(crate) fn install_server(sh: &Shell) -> anyhow::Result<()> {
    // let features = opts.malloc.to_features();
    let profile = "release";
    // let profile = if opts.dev_rel { "dev-rel" } else { "release" };

    let cmd = cmd!(
        sh,
        "cargo install --path crates/aptos-analyzer --profile={profile} --locked --force"
    );
    cmd.run()?;
    Ok(())
}
