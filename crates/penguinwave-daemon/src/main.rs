//! Penguin Wave audio daemon.

mod framing;
mod notify;
mod ratelimit;
mod session;
mod socket;
mod supervisor;

use penguinwave_core::{ConfigStore, CoreState};
use penguinwave_hid::HidApiBackend;
use penguinwave_pipewire::PactlBackend;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> std::process::ExitCode {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("penguinwave-daemon {VERSION}");
        return std::process::ExitCode::SUCCESS;
    }
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        println!("penguinwave-daemon {VERSION}\n\nusage: penguinwave-daemon [--version] [--help]");
        println!("\nListens on $XDG_RUNTIME_DIR/penguinwave/daemon.sock.");
        return std::process::ExitCode::SUCCESS;
    }

    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("penguinwave-daemon: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let path = socket::socket_path()?;
    let listener = socket::bind(&path).map_err(|e| format!("{}: {e}", path.display()))?;

    let backend = Arc::new(PactlBackend::new());
    let hid = Arc::new(HidApiBackend::new().map_err(|e| format!("hidapi unavailable: {e}"))?);
    let state = CoreState::new(backend, hid, ConfigStore::from_env());

    // A missing audio server is not a startup failure: the supervisor brings
    // the desired state up when PipeWire appears.
    if let Err(e) = state.start() {
        eprintln!("[daemon] deferred startup: {e}");
    }

    let running = Arc::new(AtomicBool::new(true));
    let supervisor = supervisor::spawn(state.clone(), running.clone());

    notify::ready();
    eprintln!("[daemon] listening on {}", path.display());

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let state = state.clone();
                std::thread::spawn(move || session::serve(stream, state, VERSION));
            }
            Err(e) => eprintln!("[daemon] accept failed: {e}"),
        }
    }

    notify::stopping();
    running.store(false, Ordering::SeqCst);
    state.shutdown();
    let _ = supervisor.join();
    let _ = std::fs::remove_file(&path);
    Ok(())
}
