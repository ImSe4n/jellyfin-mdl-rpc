//! Runs jellyfin-rpc with no console window, logging to a file. The installer's
//! Start Menu and "start at login" shortcuts point here.
//!
//! Every argument is passed through to jellyfin-rpc unchanged.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::env;
use std::fs::{self, File, TryLockError};
use std::path::PathBuf;
use std::process::{exit, Command, Stdio};

const MAIN_EXE: &str = if cfg!(windows) {
    "jellyfin-rpc.exe"
} else {
    "jellyfin-rpc"
};
const LOG_FILE: &str = "jellyfin-rpc.log";
/// Held for as long as jellyfin-rpc runs, so a second launch can tell and back off.
const LOCK_FILE: &str = "jellyfin-rpc.lock";

/// Same folder as main.json: `%APPDATA%\jellyfin-rpc` or `~/.config/jellyfin-rpc`.
fn config_dir() -> Option<PathBuf> {
    let base = if cfg!(windows) {
        PathBuf::from(env::var_os("APPDATA")?)
    } else {
        match env::var_os("XDG_CONFIG_HOME") {
            Some(dir) => PathBuf::from(dir),
            None => PathBuf::from(env::var_os("HOME")?).join(".config"),
        }
    };
    Some(base.join("jellyfin-rpc"))
}

enum Outcome {
    Exited(i32),
    AlreadyRunning,
}

fn run() -> Result<Outcome, String> {
    let exe = env::current_exe().map_err(|err| format!("cannot locate launcher: {err}"))?;
    let main_exe = exe
        .parent()
        .ok_or("launcher has no parent folder")?
        .join(MAIN_EXE);

    let dir = config_dir().ok_or("cannot find the config folder")?;
    fs::create_dir_all(&dir).map_err(|err| format!("cannot create {}: {err}", dir.display()))?;

    // Without a window it's easy to start it twice, and two copies fight over the
    // Discord status. Lock before touching the log so the running copy's log survives.
    let lock_path = dir.join(LOCK_FILE);
    let lock = File::create(&lock_path)
        .map_err(|err| format!("cannot write {}: {err}", lock_path.display()))?;
    match lock.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => return Ok(Outcome::AlreadyRunning),
        Err(TryLockError::Error(err)) => {
            return Err(format!("cannot lock {}: {err}", lock_path.display()))
        }
    }

    let log_path = dir.join(LOG_FILE);
    // Overwritten on each start so it never grows without bound.
    let log = File::create(&log_path)
        .map_err(|err| format!("cannot write {}: {err}", log_path.display()))?;
    let log_err = log
        .try_clone()
        .map_err(|err| format!("cannot share log file: {err}"))?;

    let mut command = Command::new(&main_exe);
    command
        .args(env::args_os().skip(1))
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .stdout(log)
        .stderr(log_err);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let status = command
        .status()
        .map_err(|err| format!("cannot start {}: {err}", main_exe.display()))?;
    drop(lock);
    Ok(Outcome::Exited(status.code().unwrap_or(1)))
}

fn main() {
    match run() {
        Ok(Outcome::Exited(code)) => exit(code),
        Ok(Outcome::AlreadyRunning) => exit(0),
        Err(reason) => {
            // No console to print to; leave the reason where the log would have been.
            if let Some(dir) = config_dir() {
                let _ = fs::create_dir_all(&dir);
                let _ = fs::write(dir.join(LOG_FILE), format!("jellyfin-rpc-background: {reason}\n"));
            }
            exit(1)
        }
    }
}
