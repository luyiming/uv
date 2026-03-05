//! Host system information (OS type, kernel release, distro metadata).

/// Returns the operating system type (e.g., `"Linux"`, `"Darwin"`, `"Windows_NT"`).
pub fn os_type() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        fs_err::read_to_string("/proc/sys/kernel/ostype")
            .ok()
            .map(|s| s.trim().to_string())
    }
    #[cfg(target_os = "macos")]
    {
        Some("Darwin".to_string())
    }
    #[cfg(target_os = "windows")]
    {
        Some("Windows_NT".to_string())
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        None
    }
}

/// Returns the OS kernel release version string.
///
/// On Unix this is the equivalent of `uname -r` (e.g., `"6.8.0-90-generic"`).
/// On Windows this reads `CurrentBuildNumber` from the registry.
pub fn os_release() -> Option<String> {
    #[cfg(unix)]
    {
        let uname = rustix::system::uname();
        let release = uname.release().to_str().ok()?;
        Some(release.to_string())
    }
    #[cfg(windows)]
    {
        windows_os_release()
    }
    #[cfg(not(any(unix, windows)))]
    {
        None
    }
}

/// Read the Windows build number from the registry.
#[cfg(windows)]
fn windows_os_release() -> Option<String> {
    let key = windows_registry::LOCAL_MACHINE
        .open(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")
        .ok()?;
    key.get_string("CurrentBuildNumber").ok()
}

/// Parsed fields from `/etc/os-release`.
#[derive(Debug, Clone, Default)]
pub struct LinuxOsRelease {
    /// Distribution name (e.g., `"Ubuntu"`).
    pub name: Option<String>,
    /// Version identifier (e.g., `"22.04"`).
    pub version_id: Option<String>,
    /// Version codename (e.g., `"jammy"`).
    pub version_codename: Option<String>,
}

/// Parses `/etc/os-release` on Linux and returns selected fields.
///
/// Returns `None` on non-Linux platforms or if the file cannot be read.
pub fn linux_os_release() -> Option<LinuxOsRelease> {
    #[cfg(target_os = "linux")]
    {
        Some(parse_os_release(
            &fs_err::read_to_string("/etc/os-release").ok()?,
        ))
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Parse the contents of an os-release file (KEY=VALUE format, optionally quoted).
#[cfg(any(target_os = "linux", test))]
fn parse_os_release(contents: &str) -> LinuxOsRelease {
    let mut release = LinuxOsRelease::default();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = unquote(value);
        match key {
            "NAME" => release.name = Some(value.to_string()),
            "VERSION_ID" => release.version_id = Some(value.to_string()),
            "VERSION_CODENAME" => release.version_codename = Some(value.to_string()),
            _ => {}
        }
    }
    release
}

/// Strip matching single or double quotes from a value.
#[cfg(any(target_os = "linux", test))]
fn unquote(s: &str) -> &str {
    for quote in ['"', '\''] {
        if let Some(inner) = s.strip_prefix(quote).and_then(|s| s.strip_suffix(quote)) {
            return inner;
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_os_release_ubuntu() {
        let contents = "\
NAME=\"Ubuntu\"
VERSION_ID=\"22.04\"
VERSION_CODENAME=jammy
ID=ubuntu
";
        let release = parse_os_release(contents);
        assert_eq!(release.name.as_deref(), Some("Ubuntu"));
        assert_eq!(release.version_id.as_deref(), Some("22.04"));
        assert_eq!(release.version_codename.as_deref(), Some("jammy"));
    }

    #[test]
    fn test_parse_os_release_empty() {
        let release = parse_os_release("");
        assert_eq!(release.name, None);
        assert_eq!(release.version_id, None);
        assert_eq!(release.version_codename, None);
    }

    #[test]
    fn test_parse_os_release_comments_and_blanks() {
        let contents = "\
# This is a comment

NAME='Fedora Linux'
VERSION_ID=40
";
        let release = parse_os_release(contents);
        assert_eq!(release.name.as_deref(), Some("Fedora Linux"));
        assert_eq!(release.version_id.as_deref(), Some("40"));
        assert_eq!(release.version_codename, None);
    }

    #[test]
    fn test_unquote() {
        assert_eq!(unquote("\"hello\""), "hello");
        assert_eq!(unquote("'hello'"), "hello");
        assert_eq!(unquote("hello"), "hello");
        assert_eq!(unquote("\"\""), "");
        assert_eq!(unquote(""), "");
    }

    #[test]
    fn test_os_type_returns_value() {
        // On any supported platform, os_type should return Some.
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
        assert!(os_type().is_some());
    }

    #[test]
    fn test_os_release_returns_value() {
        // On any supported platform, os_release should return Some.
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
        assert!(os_release().is_some());
    }
}
