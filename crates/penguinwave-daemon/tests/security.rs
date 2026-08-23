//! The socket's permissions are its only authentication.

#[path = "../src/transport/socket.rs"]
mod socket;

use std::os::unix::fs::PermissionsExt;

fn temp(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("penguinwave-sec-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// Both env cases live in one test: they mutate the same process-wide
/// variable, and separate tests would race under the parallel runner.
#[test]
fn the_socket_path_requires_xdg_runtime_dir() {
    let saved = std::env::var_os("XDG_RUNTIME_DIR");

    unsafe { std::env::remove_var("XDG_RUNTIME_DIR") };
    let refused = socket::socket_path();

    unsafe { std::env::set_var("XDG_RUNTIME_DIR", "/run/user/1234") };
    let accepted = socket::socket_path();

    match saved {
        Some(v) => unsafe { std::env::set_var("XDG_RUNTIME_DIR", v) },
        None => unsafe { std::env::remove_var("XDG_RUNTIME_DIR") },
    }

    // Absent XDG_RUNTIME_DIR must be a refusal, never a /tmp fallback: a
    // world-writable directory would hand local audio control to any user.
    let err = refused.unwrap_err();
    assert!(err.contains("XDG_RUNTIME_DIR"));
    assert!(!err.contains("/tmp"));

    assert_eq!(
        accepted.unwrap(),
        std::path::Path::new("/run/user/1234/penguinwave/daemon.sock")
    );
}

#[test]
fn bind_creates_a_private_directory_and_socket() {
    let dir = temp("perms");
    let path = dir.join("penguinwave").join("daemon.sock");

    let _listener = socket::bind(&path).unwrap();

    let dir_mode = std::fs::metadata(path.parent().unwrap())
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    let sock_mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(
        dir_mode, 0o700,
        "directory must not be group/other readable"
    );
    assert_eq!(sock_mode, 0o600, "socket must not be group/other readable");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_stale_socket_file_is_replaced() {
    let dir = temp("stale");
    let path = dir.join("penguinwave").join("daemon.sock");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"stale").unwrap();

    assert!(socket::bind(&path).is_ok());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_second_daemon_refuses_to_take_the_socket() {
    let dir = temp("inuse");
    let path = dir.join("penguinwave").join("daemon.sock");
    let _first = socket::bind(&path).unwrap();

    let err = socket::bind(&path).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::AddrInUse);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_overlong_socket_path_is_refused_with_a_usable_message() {
    let deep = temp("deep").join("x".repeat(120));
    let path = deep.join("daemon.sock");

    let err = socket::bind(&path).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert!(err.to_string().contains("XDG_RUNTIME_DIR"));
}
