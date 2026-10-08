// SPDX-License-Identifier: LGPL-3.0-or-later
use super::security::{self, Config, ROOT, STATE};
use anyhow::{Result, ensure};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::Path,
    process::Command,
};
const SHARE: &str = "/usr/local/share/gxfp51b7-guest";
const DISK: &str = "/var/lib/gxfp51b7-vm";
const ASSETS: [&str; 6] = [
    "le.signed.sgxs",
    "le.production.sigstruct",
    "WBDI_Enclave.signed.sgxs",
    "WBDI_Enclave.signed.sigstruct",
    "white_list_cert.bin",
    "chicago-default-config.bin",
];
fn regular(path: &Path) -> Result<()> {
    ensure!(
        fs::symlink_metadata(path)?.is_file(),
        "Expected regular file: {}",
        path.display()
    );
    Ok(())
}
fn digest(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn directory(path: &Path, mode: u32) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    security::trusted(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    Ok(())
}
fn copy(source: &Path, target: &Path, mode: u32) -> Result<()> {
    regular(source)?;
    ensure!(
        !target.exists() && fs::symlink_metadata(target).is_err(),
        "Installation target already exists"
    );
    fs::copy(source, target)?;
    fs::set_permissions(target, fs::Permissions::from_mode(mode))?;
    Ok(())
}
pub fn install(bundle: &Path, user: &str, source: &Path) -> Result<()> {
    security::root()?;
    let user = security::account(user)?;
    for path in [
        ROOT,
        STATE,
        SHARE,
        DISK,
        "/etc/systemd/system/gxfp51b7-vm.service",
        "/usr/lib/security/pam_gxfp51b7.so",
        "/etc/pam.d/gxfp51b7-test",
    ] {
        ensure!(
            fs::symlink_metadata(path).is_err(),
            "Existing deployment requires migration: {path}"
        );
    }
    ensure!(
        fs::read_to_string("/sys/class/dmi/id/sys_vendor")?.trim() == "HUAWEI"
            && fs::read_to_string("/sys/class/dmi/id/product_name")?.trim() == "MACHC-WAX9"
            && Path::new("/sys/bus/acpi/devices/GXFP51B7:00").exists(),
        "Expected HUAWEI MACHC-WAX9 with GXFP51B7"
    );
    let bundle = bundle.canonicalize()?;
    let source = source.canonicalize()?;
    regular(&bundle.join("assets.json"))?;
    let manifest: Value = serde_json::from_reader(fs::File::open(bundle.join("assets.json"))?)?;
    for name in ASSETS {
        let path = bundle.join(name);
        regular(&path)?;
        ensure!(
            manifest[name].as_str() == Some(digest(&path)?.as_str()),
            "Asset digest mismatch: {name}"
        );
    }
    for name in ["id_ed25519", "known_hosts", "guest.qcow2"] {
        regular(&bundle.join(name))?;
    }
    ensure!(
        fs::read_to_string(bundle.join("known_hosts"))?
            .lines()
            .any(|l| l.starts_with("[127.0.0.1]:2228 ")),
        "Pin the guest key for [127.0.0.1]:2228"
    );
    ensure!(
        fs::metadata(bundle.join("chicago-default-config.bin"))?.len() == 256,
        "Unexpected sensor configuration"
    );
    let disk = Command::new("/usr/bin/qemu-img")
        .args(["info", "--output=json"])
        .arg(bundle.join("guest.qcow2"))
        .output()?;
    ensure!(disk.status.success(), "Guest disk inspection failed");
    let info: Value = serde_json::from_slice(&disk.stdout)?;
    ensure!(
        info["format"] == "qcow2" && info.get("backing-filename").is_none(),
        "Expected a stopped self-contained qcow2 guest"
    );
    for name in ["gxfp51b7", "pam_gxfp51b7.so", "pam_probe", "pam_sddm_probe"] {
        regular(&source.join("build").join(name))?;
    }
    let service = if let Some(user) = nix::unistd::User::from_name("gxfpvm")? {
        ensure!(!user.uid.is_root(), "Unsafe VM account");
        user
    } else {
        ensure!(
            Command::new("/usr/bin/useradd")
                .args([
                    "--system",
                    "--user-group",
                    "--home-dir",
                    DISK,
                    "--shell",
                    "/usr/bin/nologin",
                    "--groups",
                    "kvm,sgx",
                    "gxfpvm"
                ])
                .status()?
                .success(),
            "Creating VM account failed"
        );
        nix::unistd::User::from_name("gxfpvm")?
            .ok_or_else(|| anyhow::anyhow!("Missing VM account"))?
    };
    directory(Path::new(ROOT), 0o755)?;
    directory(Path::new(STATE), 0o700)?;
    directory(Path::new(DISK), 0o750)?;
    let private = Path::new(ROOT).join("guest-private");
    directory(&private, 0o700)?;
    directory(&Path::new(SHARE).join("sgxs"), 0o755)?;
    directory(&Path::new(SHARE).join("windows-psw218"), 0o755)?;
    for name in ["id_ed25519", "known_hosts", "chicago-default-config.bin"] {
        copy(&bundle.join(name), &private.join(name), 0o600)?;
    }
    for name in ASSETS {
        if name == "chicago-default-config.bin" {
            continue;
        }
        let dir = if name == "white_list_cert.bin" {
            "windows-psw218"
        } else {
            "sgxs"
        };
        let target = Path::new(SHARE).join(dir).join(name);
        copy(&bundle.join(name), &target, 0o644)?;
        ensure!(
            manifest[name].as_str() == Some(digest(&target)?.as_str()),
            "Asset changed during installation"
        );
    }
    ensure!(
        manifest["chicago-default-config.bin"].as_str()
            == Some(digest(&private.join("chicago-default-config.bin"))?.as_str()),
        "Configuration changed during installation"
    );
    copy(
        &bundle.join("guest.qcow2"),
        &Path::new(DISK).join("guest.qcow2"),
        0o600,
    )?;
    nix::unistd::chown(
        &Path::new(DISK).join("guest.qcow2"),
        Some(service.uid),
        Some(service.gid),
    )?;
    nix::unistd::chown(Path::new(DISK), Some(service.uid), Some(service.gid))?;
    for name in ["gxfp51b7", "pam_probe", "pam_sddm_probe"] {
        copy(
            &source.join("build").join(name),
            &Path::new(ROOT).join(name),
            0o755,
        )?;
    }
    copy(
        &source.join("build/pam_gxfp51b7.so"),
        Path::new("/usr/lib/security/pam_gxfp51b7.so"),
        0o644,
    )?;
    let unit = include_bytes!("../../../data/gxfp51b7-vm.service");
    super::admin::atomic_write(
        Path::new("/etc/systemd/system/gxfp51b7-vm.service"),
        unit,
        0o644,
    )?;
    super::admin::save(
        "config.json",
        &Config {
            enabled: false,
            user: user.name.clone(),
            uid: user.uid.as_raw(),
            policy: "experimental-affine-ncc-v3".into(),
            threshold: 0.86,
            reference_count: 15,
            experimental: true,
        },
    )?;
    let test = format!(
        "auth required pam_shells.so\nauth requisite pam_nologin.so\nauth requisite pam_faillock.so preauth\nauth required pam_gxfp51b7.so user={}\naccount include system-login\n",
        user.name
    );
    super::admin::atomic_write(
        Path::new("/etc/pam.d/gxfp51b7-test"),
        test.as_bytes(),
        0o644,
    )?;
    ensure!(
        Command::new("/usr/bin/systemctl")
            .arg("daemon-reload")
            .status()?
            .success(),
        "Service reload failed"
    );
    ensure!(fs::metadata(ROOT)?.uid() == 0, "Unsafe runtime ownership");
    println!("Rust runtime installed. Start the VM, enroll, check and enable SDDM.");
    Ok(())
}
