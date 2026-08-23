//! The EQ filter-chain child process.
//!
//! Behind the backend trait because it is a subprocess: a native binding
//! implementation would build the chain a different way, and a mock can fake
//! a crash loop without spawning anything.

use penguinwave_proto::{ErrorKind, PwError};
use std::path::Path;
use std::process::{Child, Command, Stdio};

/// A running filter chain.
pub trait ChainProcess: Send {
    /// `Ok(None)` while still running, `Ok(Some(_))` once it has exited.
    fn try_wait(&mut self) -> Result<Option<i32>, PwError>;
    fn kill(&mut self);
}

pub(crate) fn spawn(conf_path: &Path) -> Result<Box<dyn ChainProcess>, PwError> {
    // PR_SET_PDEATHSIG below should kill an old chain along with its daemon,
    // but that signal tracks the exact thread that called it, not the whole
    // process -- on a multi-threaded async runtime it can be silently missed.
    // Reap any chain still running the same conf as a fallback so a killed
    // daemon never leaves a duplicate chain behind on the next start.
    reap_stale(conf_path);

    let mut command = Command::new("pipewire");
    command
        .arg("-c")
        .arg(conf_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    // Kill the child with the parent even if the daemon dies without a
    // shutdown path.
    #[cfg(target_os = "linux")]
    unsafe {
        use std::os::unix::process::CommandExt;
        command.pre_exec(|| {
            libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
            Ok(())
        });
    }

    let child = command.spawn().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => PwError::new(
            ErrorKind::PipeWireUnavailable,
            "pipewire not found; is PipeWire installed?",
        ),
        _ => PwError::new(
            ErrorKind::PipeWireFailed,
            format!("failed to spawn filter chain: {e}"),
        ),
    })?;

    Ok(Box::new(SpawnedChain { child }))
}

#[cfg(target_os = "linux")]
fn reap_stale(conf_path: &Path) {
    let _ = Command::new("pkill")
        .arg("-f")
        .arg(format!("pipewire -c {}", conf_path.display()))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(not(target_os = "linux"))]
fn reap_stale(_conf_path: &Path) {}

struct SpawnedChain {
    child: Child,
}

impl ChainProcess for SpawnedChain {
    fn try_wait(&mut self) -> Result<Option<i32>, PwError> {
        self.child
            .try_wait()
            .map(|s| s.map(|s| s.code().unwrap_or(-1)))
            .map_err(|e| PwError::new(ErrorKind::PipeWireFailed, e.to_string()))
    }

    fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
