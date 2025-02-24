use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;

pub fn command(cmd: impl AsRef<OsStr>, working_directory: impl AsRef<Path>) -> Command {
    // we are `toolchain::command``
    #[allow(clippy::disallowed_methods)]
    let mut cmd = Command::new(cmd);
    cmd.current_dir(working_directory);
    cmd
}
