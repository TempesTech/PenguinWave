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
