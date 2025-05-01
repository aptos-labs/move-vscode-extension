use camino::Utf8PathBuf;
use std::path::Path;
use std::process::Command;

pub fn command(cmd: &Utf8PathBuf, working_directory: impl AsRef<Path>) -> Command {
    let normalized_cmd = shellexpand::tilde(cmd.as_str()).to_string();
    let mut cmd = Command::new(normalized_cmd);
    cmd.current_dir(working_directory);
    cmd
}
