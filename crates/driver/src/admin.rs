// SPDX-License-Identifier: LGPL-3.0-or-later
use super::security::{self, Config, ROOT, STATE};
use anyhow::{Result, ensure};
use gxfp_core::{image, template::Template};
use ndarray::{Array1, Array3, s};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read, Write},
    os::unix::fs::PermissionsExt,
    path::Path,
    process::Command,
    time::Duration,
};

const BEGIN: &str = "# BEGIN GXFP51B7 fingerprint login\n";
const END: &str = "# END GXFP51B7 fingerprint login\n";
pub fn valid_user(user: &str) -> bool {
    !user.is_empty()
        && user != "root"
        && user.bytes().enumerate().all(|(i, b)| {
            b.is_ascii_lowercase()
                || b == b'_'
                || (i > 0
                    && (b.is_ascii_digit() || b == b'-' || (b == b'$' && i + 1 == user.len())))
        })
}
pub fn enable_text(text: &str, user: &str) -> Result<String> {
    ensure!(valid_user(user), "Expected a regular local account name");
    ensure!(
        !text.contains(BEGIN) && !text.contains(END),
        "Fingerprint block already present or incomplete"
    );
    let first = text
        .lines()
        .find(|line| {
            !line.trim_start().starts_with('#')
                && line
                    .split_whitespace()
                    .next()
                    .is_some_and(|t| t.trim_start_matches('-') == "auth")
        })
        .ok_or_else(|| anyhow::anyhow!("Missing authentication layout"))?;
    ensure!(
        first
            .split_whitespace()
            .eq(["auth", "include", "system-login"]),
        "Expected Arch system-login layout"
    );
    let block = format!(
        "{BEGIN}auth required pam_shells.so\nauth requisite pam_nologin.so\nauth required pam_env.so\nauth requisite pam_faillock.so preauth\nauth [success=ok default=1] pam_gxfp51b7.so user={user}\nauth sufficient pam_faillock.so authsucc\n{END}"
    );
    let index = text
        .lines()
        .take_while(|line| *line != first)
        .map(|line| line.len() + 1)
        .sum::<usize>();
    Ok(format!("{}{}{}", &text[..index], block, &text[index..]))
}
pub fn disable_text(text: &str) -> Result<String> {
    if !text.contains(BEGIN) && !text.contains(END) {
        return Ok(text.to_owned());
    }
    ensure!(
        text.matches(BEGIN).count() == 1 && text.matches(END).count() == 1,
        "Incomplete or ambiguous fingerprint block"
    );
    let start = text.find(BEGIN).unwrap();
    let stop = text.find(END).unwrap();
    ensure!(stop > start, "Invalid block order");
    Ok(format!("{}{}", &text[..start], &text[stop + END.len()..]))
}
pub fn atomic_write(path: &Path, data: &[u8], mode: u32) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Missing parent"))?;
    security::trusted(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary
        .as_file()
        .set_permissions(fs::Permissions::from_mode(mode))?;
    temporary.write_all(data)?;
    temporary.as_file().sync_all()?;
    temporary.persist(path)?;
    fs::File::open(parent)?.sync_all()?;
    Ok(())
}
pub fn save(name: &str, value: &impl serde::Serialize) -> Result<()> {
    let mut data = serde_json::to_vec_pretty(value)?;
    data.push(b'\n');
    atomic_write(&Path::new(STATE).join(name), &data, 0o600)
}
fn installed() -> Result<()> {
    security::root()?;
    let executable = std::env::current_exe()?;
    ensure!(
        executable == Path::new(ROOT).join("gxfp51b7"),
        "Use the installed root-owned administrator command"
    );
    security::trusted(&executable)?;
    security::trusted(Path::new(STATE))?;
    Ok(())
}
fn active() -> Result<()> {
    ensure!(
        Command::new("/usr/bin/systemctl")
            .args(["is-active", "--quiet", "gxfp51b7-vm.service"])
            .status()?
            .success(),
        "Start the isolated runtime first"
    );
    Ok(())
}
fn prompt(message: &str) -> Result<()> {
    print!("{message} ");
    io::stdout().flush()?;
    let mut reply = String::new();
    ensure!(
        io::stdin().read_line(&mut reply)? > 0,
        "Enrollment cancelled"
    );
    Ok(())
}
fn remove_report() -> Result<()> {
    let path = Path::new(STATE).join("live-validation.json");
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}
fn fingerprint() -> Result<String> {
    let mut hash = Sha256::new();
    for path in [
        Path::new(STATE).join("config.json"),
        Path::new(STATE).join("template.npz"),
        Path::new(ROOT).join("gxfp51b7"),
        Path::new("/usr/lib/security/pam_gxfp51b7.so").to_owned(),
        Path::new(ROOT).join("pam_probe"),
        Path::new("/etc/pam.d/gxfp51b7-test").to_owned(),
    ] {
        let mut file = security::open_private(&path)?;
        let mut chunk = [0; 65536];
        loop {
            let n = file.read(&mut chunk)?;
            if n == 0 {
                break;
            }
            hash.update(&chunk[..n]);
        }
    }
    Ok(format!("{:x}", hash.finalize()))
}
pub fn enroll(user: &str) -> Result<()> {
    installed()?;
    let user = security::account(user)?;
    ensure!(
        !fs::read_to_string("/etc/pam.d/sddm")?.contains(BEGIN),
        "Disable the SDDM fingerprint branch before enrollment"
    );
    active()?;
    prompt("Remove all fingers, then press Enter to capture the background.")?;
    let background = Array1::from(image::decode(&super::capture::capture(
        &super::private_directory(),
    )?)?);
    let mut images = Array3::zeros((15, 56, 72));
    for index in 0..15 {
        prompt(&format!(
            "Place the same index finger, then press Enter ({}/15).",
            index + 1
        ))?;
        let pixels = image::decode(&super::capture::capture(&super::private_directory())?)?;
        let (drop, contrast) = image::quality(&pixels, background.as_slice().unwrap())?;
        ensure!(
            drop >= 900. && contrast >= 40.,
            "Insufficient contact; previous enrollment remains available"
        );
        images
            .slice_mut(s![index, .., ..])
            .assign(&image::prepare(&pixels, background.as_slice().unwrap())?);
        prompt("Lift the finger completely, wait two seconds, then press Enter.")?;
    }
    let template = Template { background, images };
    let data = template.write(io::Cursor::new(Vec::new()))?.into_inner();
    // Invalidates the old enrollment before replacing its template atomically.
    let mut config = Config {
        enabled: false,
        user: user.name,
        uid: user.uid.as_raw(),
        policy: "experimental-affine-ncc-v3".into(),
        threshold: 0.86,
        reference_count: 15,
        experimental: true,
    };
    save("config.json", &config)?;
    atomic_write(&Path::new(STATE).join("template.npz"), &data, 0o600)?;
    config.enabled = true;
    save("config.json", &config)?;
    remove_report()?;
    println!("Enrollment stored privately. Run check before enabling SDDM.");
    Ok(())
}
pub fn check() -> Result<()> {
    installed()?;
    active()?;
    let c = security::config()?;
    security::verify_account(&c.user, &c)?;
    remove_report()?;
    let initial = fingerprint()?;
    let mut results = Vec::new();
    for (label, expected, message) in [
        ("empty", false, "Remove all fingers"),
        ("same", true, "Place the enrolled index finger"),
        ("middle", false, "Place a different middle finger"),
        ("ring", false, "Place a different ring finger"),
        ("little", false, "Place a different little finger"),
    ] {
        prompt(&format!("{message}, then press Enter."))?;
        let helper = super::validation::bounded(
            Command::new(Path::new(ROOT).join("gxfp51b7"))
                .args(["verify", &c.user])
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("LANG", "C.UTF-8"),
            Duration::from_secs(15),
        )?;
        ensure!(
            helper.code() == Some(if expected { 0 } else { 1 }),
            "Capture or matching failed for {label}"
        );
        // Check PAM as a separate invocation with the same contact maintained.
        let pam = super::validation::bounded(
            Command::new(Path::new(ROOT).join("pam_probe"))
                .arg(&c.user)
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("LANG", "C.UTF-8"),
            Duration::from_secs(18),
        )?;
        ensure!(
            pam.code() == Some(if expected { 0 } else { 1 }),
            "Unexpected PAM result for {label}"
        );
        results.push(json!({"case":label,"accepted":expected}));
        println!("Expected helper and PAM result observed.");
    }
    ensure!(
        initial == fingerprint()?,
        "Runtime changed during validation"
    );
    save(
        "live-validation.json",
        &json!({"fingerprint":initial,"results":results}),
    )?;
    println!("Five live helper/PAM checks passed. Local commissioning evidence stored privately.");
    Ok(())
}
pub fn enable() -> Result<()> {
    installed()?;
    active()?;
    let c = security::config()?;
    security::verify_account(&c.user, &c)?;
    let report: Value = serde_json::from_reader(security::open_private(
        &Path::new(STATE).join("live-validation.json"),
    )?)?;
    let expected = json!([{"case":"empty","accepted":false},{"case":"same","accepted":true},{"case":"middle","accepted":false},{"case":"ring","accepted":false},{"case":"little","accepted":false}]);
    ensure!(
        report["fingerprint"] == fingerprint()? && report["results"] == expected,
        "Fresh live validation required for this code and template"
    );
    let path = Path::new("/etc/pam.d/sddm");
    security::trusted(path)?;
    let original = fs::read_to_string(path)?;
    let updated = enable_text(&original, &c.user)?;
    let backup = Path::new(STATE).join("backups");
    if !backup.exists() {
        fs::create_dir(&backup)?;
        fs::set_permissions(&backup, fs::Permissions::from_mode(0o700))?;
    }
    security::trusted(&backup)?;
    let target = backup.join("sddm-before-fingerprint");
    if !target.exists() {
        atomic_write(&target, original.as_bytes(), 0o600)?;
    }
    atomic_write(path, updated.as_bytes(), 0o644)?;
    println!("SDDM fingerprint branch enabled. Password authentication available.");
    Ok(())
}
pub fn disable() -> Result<()> {
    installed()?;
    let path = Path::new("/etc/pam.d/sddm");
    security::trusted(path)?;
    atomic_write(
        path,
        disable_text(&fs::read_to_string(path)?)?.as_bytes(),
        0o644,
    )?;
    let mut c = security::config()?;
    c.enabled = false;
    save("config.json", &c)?;
    remove_report()?;
    ensure!(
        Command::new("/usr/bin/systemctl")
            .args(["disable", "--now", "gxfp51b7-vm.service"])
            .status()?
            .success(),
        "Stopping runtime failed"
    );
    println!("Fingerprint branch removed; VM stopped. Password authentication available.");
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    const ORIGINAL: &str = "#%PAM-1.0\n\nauth        include     system-login\n-auth optional pam_kwallet5.so\naccount include system-login\npassword include system-login\nsession include system-login\n";
    #[test]
    fn roundtrip_with_later_edits() {
        let enabled = enable_text(ORIGINAL, "testuser").unwrap();
        assert!(enabled.contains("user=testuser\n"));
        assert_eq!(
            disable_text(&(enabled + "# later edit\n")).unwrap(),
            ORIGINAL.to_owned() + "# later edit\n"
        );
    }
    #[test]
    fn rejects_injection_and_ambiguous_layout() {
        for user in [
            "root",
            "test\nauth sufficient pam_permit.so",
            "bad name",
            "1user",
            "a$b",
        ] {
            assert!(enable_text(ORIGINAL, user).is_err());
        }
        for text in [
            "auth include common-auth\n",
            "auth required pam_deny.so\nauth include system-login\n",
        ] {
            assert!(enable_text(text, "testuser").is_err());
        }
        let enabled = enable_text(ORIGINAL, "testuser").unwrap();
        assert!(enable_text(&enabled, "testuser").is_err());
        assert!(disable_text(&enabled.replace(END, "")).is_err());
        assert_eq!(disable_text(ORIGINAL).unwrap(), ORIGINAL);
    }
}
