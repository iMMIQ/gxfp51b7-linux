// SPDX-License-Identifier: LGPL-3.0-or-later
use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
use nix::{
    sys::signal::{Signal, killpg},
    unistd::{Pid, Uid},
};
use pam::{
    constants::{PAM_SILENT, PAM_TEXT_INFO, PamFlag, PamResultCode as PamError},
    conv::Conv,
    module::{PamHandle, PamHooks},
};
use std::ffi::CStr;
use std::{
    os::fd::AsFd,
    process::{Child, ExitStatus},
    time::Instant,
};
use std::{
    os::unix::process::CommandExt,
    process::{Command, Stdio},
    time::Duration,
};
struct Fingerprint;
fn authenticate(pam: &mut PamHandle, flags: PamFlag, args: Vec<&CStr>) -> PamError {
    if !Uid::effective().is_root() {
        return PamError::PAM_AUTHINFO_UNAVAIL;
    }
    let user = match pam.get_user(None) {
        Ok(user) if !user.is_empty() => user,
        _ => return PamError::PAM_AUTHINFO_UNAVAIL,
    };
    let mut users = args
        .iter()
        .filter_map(|a| a.to_str().ok().and_then(|a| a.strip_prefix("user=")));
    if users.next() != Some(user.as_str()) || users.next().is_some() {
        return PamError::PAM_AUTHINFO_UNAVAIL;
    }
    if flags & PAM_SILENT == 0
        && let Ok(Some(conv)) = pam.get_item::<Conv>()
    {
        let _ = conv.send(
            PAM_TEXT_INFO,
            "Touch the enrolled index finger (up to 15 seconds)",
        );
    }
    let mut command = Command::new("/usr/local/lib/gxfp51b7/gxfp51b7");
    command
        .args(["verify", &user])
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LANG", "C.UTF-8")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0);
    // SAFETY: pre_exec runs after fork. Only async-signal-safe libc calls run
    // here, with stack-only arguments; Rust allocation and locking are avoided.
    unsafe {
        command.pre_exec(|| {
            let limit = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            if libc::setrlimit(libc::RLIMIT_CORE, &limit) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            // Standard-library spawn keeps its own CLOEXEC error pipe until exec.
            // Mark inherited descriptors CLOEXEC so that pipe remains usable here.
            if libc::syscall(
                libc::SYS_close_range,
                3_u32,
                u32::MAX,
                libc::CLOSE_RANGE_CLOEXEC,
            ) != 0
            {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => return PamError::PAM_AUTHINFO_UNAVAIL,
    };
    let result = wait(&mut child, Duration::from_secs(15));
    let pid = Pid::from_raw(child.id() as i32);
    let _ = killpg(pid, Signal::SIGTERM);
    std::thread::sleep(Duration::from_millis(50));
    let _ = killpg(pid, Signal::SIGKILL);
    match result {
        Ok(Some(status)) if status.success() => PamError::PAM_SUCCESS,
        Ok(Some(_)) => PamError::PAM_AUTHINFO_UNAVAIL,
        _ => {
            let _ = child.kill();
            let _ = child.wait();
            PamError::PAM_AUTHINFO_UNAVAIL
        }
    }
}
/// Wait through a Linux process descriptor. PAM may be unloaded after a call,
/// so this module uses local descriptors rather than global signal handlers.
fn wait(child: &mut Child, limit: Duration) -> std::io::Result<Option<ExitStatus>> {
    let descriptor = rustix::process::pidfd_open(
        rustix::process::Pid::from_child(child),
        rustix::process::PidfdFlags::empty(),
    )?;
    let deadline = Instant::now() + limit;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            return Ok(None);
        };
        let mut descriptors = [PollFd::new(descriptor.as_fd(), PollFlags::POLLIN)];
        let timeout = PollTimeout::try_from(remaining).map_err(std::io::Error::other)?;
        match poll(&mut descriptors, timeout) {
            Ok(0) => return Ok(None),
            Ok(_) => {}
            Err(nix::errno::Errno::EINTR) => continue,
            Err(error) => return Err(error.into()),
        }
    }
}
impl PamHooks for Fingerprint {
    fn sm_authenticate(pam: &mut PamHandle, args: Vec<&CStr>, flags: PamFlag) -> PamError {
        authenticate(pam, flags, args)
    }
    fn sm_setcred(_: &mut PamHandle, _: Vec<&CStr>, _: PamFlag) -> PamError {
        PamError::PAM_SUCCESS
    }
}
pam::pam_hooks!(Fingerprint);

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn process_descriptor_deadline_and_status() {
        // SAFETY: sigaction with a null new action only reads the current
        // disposition into the valid output pointer supplied below.
        let current_handler = || unsafe {
            let mut action = std::mem::MaybeUninit::<libc::sigaction>::uninit();
            assert_eq!(
                libc::sigaction(libc::SIGCHLD, std::ptr::null(), action.as_mut_ptr()),
                0
            );
            action.assume_init().sa_sigaction
        };
        let before = current_handler();
        let mut child = Command::new("/bin/sh")
            .args(["-c", "exit 1"])
            .spawn()
            .unwrap();
        assert_eq!(
            wait(&mut child, Duration::from_secs(1))
                .unwrap()
                .unwrap()
                .code(),
            Some(1)
        );
        let mut child = Command::new("/bin/sleep").arg("5").spawn().unwrap();
        let start = Instant::now();
        assert!(
            wait(&mut child, Duration::from_millis(100))
                .unwrap()
                .is_none()
        );
        child.kill().unwrap();
        child.wait().unwrap();
        assert!(start.elapsed() < Duration::from_secs(2));
        assert_eq!(current_handler(), before);
    }
}
