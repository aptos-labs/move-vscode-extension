use anyhow::{Context, bail, format_err};
use std::env;
use std::path::PathBuf;
use xshell::{Shell, cmd};

pub(crate) fn install(client: bool, server: bool) -> anyhow::Result<()> {
    let sh = Shell::new()?;
    if cfg!(target_os = "macos") {
        fix_path_for_mac(&sh).context("Fix path for mac")?;
    }
    if server {
        install_server(&sh).context("install server")?;
    }
    if client {
        install_client(&sh).context("cannot find VSCode editor")?;
    }
    Ok(())
}

fn install_client(sh: &Shell) -> anyhow::Result<()> {
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

    let candidates = VS_CODES;
    let code = candidates
        .iter()
        .copied()
        .find(|&bin| {
            if cfg!(unix) {
                cmd!(sh, "{bin} --version").read().is_ok()
            } else {
                cmd!(sh, "cmd.exe /c {bin}.cmd --version").read().is_ok()
            }
        })
        .ok_or_else(|| {
            format_err!(
                "Can't execute `{} --version`. Perhaps it is not in $PATH?\n\
                NOTE:\nTo install the extension for the other editors (ie. Cursor IDE),\n\
                use \"Install from VSIX...\" command with the `./editors/code/aptos-analyzer.vsix` extension file",
                candidates[0]
            )
        })?;

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

fn install_server(sh: &Shell) -> anyhow::Result<()> {
    let profile = "release";
    let cmd = cmd!(
        sh,
        "cargo install --path crates/aptos-analyzer --profile={profile} --locked --force"
    );
    cmd.run()?;
    Ok(())
}

const VS_CODES: &[&str] = &["code", "code-exploration", "code-insiders", "codium", "code-oss"];

fn fix_path_for_mac(sh: &Shell) -> anyhow::Result<()> {
    let mut vscode_path: Vec<PathBuf> = {
        const COMMON_APP_PATH: &str = r"/Applications/Visual Studio Code.app/Contents/Resources/app/bin";
        const ROOT_DIR: &str = "";
        let home_dir = sh
            .var("HOME")
            .map_err(|err| format_err!("Failed getting HOME from environment with error: {}.", err))?;

        [ROOT_DIR, &home_dir]
            .into_iter()
            .map(|dir| dir.to_owned() + COMMON_APP_PATH)
            .map(PathBuf::from)
            .filter(|path| path.exists())
            .collect()
    };

    if !vscode_path.is_empty() {
        let vars = sh
            .var_os("PATH")
            .context("Could not get PATH variable from env.")?;

        let mut paths = env::split_paths(&vars).collect::<Vec<_>>();
        paths.append(&mut vscode_path);
        let new_paths = env::join_paths(paths).context("build env PATH")?;
        sh.set_var("PATH", new_paths);
    }

    Ok(())
}
