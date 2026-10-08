// SPDX-License-Identifier: LGPL-3.0-or-later
use anyhow::{Result, bail};
use nix::{
    sys::signal::{Signal, killpg},
    unistd::Pid,
};
use std::{
    os::unix::process::CommandExt,
    process::{Command, ExitStatus, Stdio},
    time::Duration,
};
use wait_timeout::ChildExt;

/// Bound the entire helper process group, including its SSH subprocess.
pub fn bounded(command: &mut Command, limit: Duration) -> Result<ExitStatus> {
    let mut child = command
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let result = child.wait_timeout(limit);
    let pid = Pid::from_raw(child.id().try_into()?);
    let _ = killpg(pid, Signal::SIGTERM);
    std::thread::sleep(Duration::from_millis(50));
    let _ = killpg(pid, Signal::SIGKILL);
    match result {
        Ok(Some(status)) => Ok(status),
        other => {
            let _ = child.kill();
            let _ = child.wait();
            match other {
                Err(e) => Err(e.into()),
                _ => bail!("Authentication helper timed out"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deadline_and_status() {
        assert_eq!(
            bounded(
                Command::new("/bin/sh").args(["-c", "exit 1"]),
                Duration::from_secs(1)
            )
            .unwrap()
            .code(),
            Some(1)
        );
        let started = std::time::Instant::now();
        assert!(
            bounded(
                Command::new("/bin/sh").args(["-c", "sleep 5"]),
                Duration::from_millis(100)
            )
            .is_err()
        );
        assert!(started.elapsed() < Duration::from_secs(2));
    }
}
