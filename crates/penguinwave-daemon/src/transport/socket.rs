//! Socket path, permissions, and bind.

use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;

/// Where the daemon listens.
///
/// `$XDG_RUNTIME_DIR` is required and there is no `/tmp` fallback: filesystem
/// permissions are the only authentication on this socket, so a world-writable
/// directory would hand local audio control to any local user.
pub fn socket_path() -> Result<PathBuf, String> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").ok_or_else(|| {
        "XDG_RUNTIME_DIR is not set; refusing to fall back to a world-readable directory"
            .to_string()
    })?;
    Ok(PathBuf::from(runtime)
        .join("penguinwave")
        .join("daemon.sock"))
}

/// `sockaddr_un.sun_path` is 108 bytes including the terminator.
const MAX_SOCKET_PATH: usize = 107;

/// Bind the listener, creating the directory `0700` and the socket `0600`.
pub fn bind(path: &std::path::Path) -> io::Result<UnixListener> {
    // The kernel's own message for this is "path must be shorter than
    // SUN_LEN", which does not say what to do about it.
    if path.as_os_str().len() > MAX_SOCKET_PATH {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "socket path is {} bytes, over the {MAX_SOCKET_PATH}-byte unix socket limit; \
                 XDG_RUNTIME_DIR is too deep",
                path.as_os_str().len()
            ),
        ));
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
    }

    // A socket file left by a crashed daemon would make bind fail; a live one
    // means another daemon owns it, which connect() below detects first.
    if path.exists() {
        if std::os::unix::net::UnixStream::connect(path).is_ok() {
            return Err(io::Error::new(
                io::ErrorKind::AddrInUse,
                "another penguinwave-daemon is already listening",
            ));
        }
        fs::remove_file(path)?;
    }

    let listener = UnixListener::bind(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(listener)
}
