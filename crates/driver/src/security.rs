// SPDX-License-Identifier: LGPL-3.0-or-later
use anyhow::{Result, ensure};
use nix::sys::resource::{Resource, setrlimit};
use nix::unistd::{Uid, User};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};

pub const ROOT: &str = "/usr/local/lib/gxfp51b7";
pub const STATE: &str = "/var/lib/gxfp51b7";
#[derive(Serialize, Deserialize)]
pub struct Config {
    pub enabled: bool,
    pub user: String,
    pub uid: u32,
    pub policy: String,
    pub threshold: f64,
    pub reference_count: usize,
    pub experimental: bool,
}
pub fn root() -> Result<()> {
    ensure!(Uid::effective().is_root(), "Requires root");
    setrlimit(Resource::RLIMIT_CORE, 0, 0)?;
    Ok(())
}
pub fn trusted(path: &Path) -> Result<()> {
    for component in path.ancestors() {
        let m = fs::symlink_metadata(component)?;
        ensure!(
            m.uid() == 0 && m.mode() & 0o022 == 0 && !m.file_type().is_symlink(),
            "Unsafe ownership or permissions: {}",
            component.display()
        );
    }
    Ok(())
}
pub fn open_private(path: &Path) -> Result<File> {
    trusted(path)?;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW)
        .open(path)?;
    let m = file.metadata()?;
    ensure!(
        m.uid() == 0 && m.mode() & 0o022 == 0 && m.is_file(),
        "Unsafe file"
    );
    Ok(file)
}
pub fn config() -> Result<Config> {
    Ok(serde_json::from_reader(open_private(
        &Path::new(STATE).join("config.json"),
    )?)?)
}
pub fn account(user: &str) -> Result<User> {
    ensure!(super::admin::valid_user(user), "Invalid local account name");
    let user = User::from_name(user)?.ok_or_else(|| anyhow::anyhow!("Unknown local user"))?;
    ensure!(!user.uid.is_root(), "Expected regular local account");
    Ok(user)
}
pub fn verify_account(user: &str, c: &Config) -> Result<()> {
    let account = account(user)?;
    ensure!(
        c.enabled
            && account.name == c.user
            && account.uid.as_raw() == c.uid
            && c.policy == "experimental-affine-ncc-v3"
            && c.threshold == 0.86
            && c.reference_count == 15,
        "Inactive or incompatible enrollment"
    );
    let shadow = fs::read_to_string("/etc/shadow")?;
    let password = shadow
        .lines()
        .find_map(|line| {
            let mut parts = line.split(':');
            if parts.next() == Some(user) {
                parts.next()
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow::anyhow!("Missing shadow entry"))?;
    ensure!(
        !password.is_empty() && !password.starts_with(['!', '*']),
        "Account password is locked"
    );
    Ok(())
}
