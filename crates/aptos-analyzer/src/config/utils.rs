use camino::Utf8PathBuf;

#[cfg(target_os = "windows")]
const FORMATTER_EXE: &str = "movefmt.exe";
#[cfg(not(target_os = "windows"))]
const FORMATTER_EXE: &str = "movefmt";

pub(super) fn find_movefmt_path() -> Option<Utf8PathBuf> {
    // See if it is present in the path where we usually install additional binaries.
    let path = get_additional_binaries_dir().join("movefmt");
    if path.exists() && path.is_file() {
        return Some(path);
    }

    // See if we can find the binary in the PATH.
    if let Some(path) = pathsearch::find_executable_in_path(FORMATTER_EXE) {
        return Utf8PathBuf::from_path_buf(path).ok();
    }

    None
}

/// Some functionality of the Aptos CLI relies on some additional binaries. This is
/// where we install them by default. These paths align with the installation script,
/// which is generally how the Linux and Windows users install the CLI.
fn get_additional_binaries_dir() -> Utf8PathBuf {
    #[cfg(windows)]
    {
        let home_dir = std::env::var("USERPROFILE").unwrap_or_else(|_| "".into());
        Utf8PathBuf::from(home_dir).join(".aptoscli/bin")
    }

    #[cfg(not(windows))]
    {
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| "".into());
        Utf8PathBuf::from(home_dir).join(".local/bin")
    }
}
