// SPDX-License-Identifier: LGPL-3.0-or-later
use super::security::{self, Config, ROOT, STATE};
use anyhow::{Result, ensure};
use gxfp_core::image;
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
pub(crate) fn valid_user(user: &str) -> bool {
    !user.is_empty()
        && user != "root"
        && user.bytes().enumerate().all(|(i, b)| {
            b.is_ascii_lowercase()
                || b == b'_'
                || (i > 0
                    && (b.is_ascii_digit() || b == b'-' || (b == b'$' && i + 1 == user.len())))
        })
}
pub(crate) fn enable_text(text: &str, user: &str, module: &Path) -> Result<String> {
    ensure!(
        module == Path::new("/usr/lib/security/pam_fprintd.so")
            || module == Path::new(ROOT).join("pam_fprintd.so"),
        "Standard PAM module required"
    );
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
        "{BEGIN}auth required pam_shells.so\nauth requisite pam_nologin.so\nauth required pam_env.so\nauth requisite pam_faillock.so preauth\nauth [success=ok default=1] {} max-tries=1 timeout=20\nauth sufficient pam_faillock.so authsucc\n{END}",
        module.display()
    );
    let index = text
        .lines()
        .take_while(|line| *line != first)
        .map(|line| line.len() + 1)
        .sum::<usize>();
    Ok(format!("{}{}{}", &text[..index], block, &text[index..]))
}
pub(crate) fn disable_text(text: &str) -> Result<String> {
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
pub(crate) fn atomic_write(path: &Path, data: &[u8], mode: u32) -> Result<()> {
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
pub(crate) fn save(name: &str, value: &impl serde::Serialize) -> Result<()> {
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
pub(crate) fn pam_module() -> Result<std::path::PathBuf> {
    let local = Path::new(ROOT).join("pam_fprintd.so");
    let path = if local.exists() {
        local
    } else {
        Path::new("/usr/lib/security/pam_fprintd.so").to_owned()
    };
    security::trusted(&path)?;
    Ok(path)
}
pub(crate) fn test_service(module: &Path) -> String {
    format!(
        "auth required {} max-tries=1 timeout=20\naccount include system-login\n",
        module.display()
    )
}
fn fingerprint() -> Result<String> {
    let c = security::config()?;
    let mut service = String::new();
    security::open_private(Path::new("/etc/pam.d/gxfp51b7-test"))?
        .take(8192)
        .read_to_string(&mut service)?;
    ensure!(
        service == test_service(&pam_module()?),
        "Use the standard PAM test service for the selected module"
    );
    let gallery = Path::new("/var/lib/fprint")
        .join(&c.user)
        .join("gxfp51b7/gxfp51b7-ec-chicagohs");
    security::trusted(&gallery)?;
    let mut prints = Vec::new();
    for entry in fs::read_dir(&gallery)? {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid print name"))?;
        let finger: u8 = name.parse()?;
        ensure!(
            (1..=10).contains(&finger) && path.metadata()?.len() <= 4 * 1024 * 1024,
            "Bounded fprintd print required"
        );
        prints.push(path);
    }
    ensure!(
        !prints.is_empty() && prints.len() <= 10,
        "Enroll a fingerprint with fprintd first"
    );
    prints.sort();
    let mut paths = vec![
        Path::new(STATE).join("config.json"),
        super::background_path(),
        Path::new(ROOT).join("gxfp51b7"),
        pam_module()?,
        Path::new(security::ADAPTER).to_owned(),
        Path::new(ROOT).join("pam_probe"),
        Path::new("/etc/pam.d/gxfp51b7-test").to_owned(),
    ];
    paths.extend(prints);
    let mut hash = Sha256::new();
    for path in paths {
        // A length prefix binds file boundaries as well as the private contents.
        let mut file = security::open_private(&path)?;
        hash.update(path.as_os_str().as_encoded_bytes());
        hash.update(file.metadata()?.len().to_le_bytes());
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
pub(crate) fn calibrate(user: &str) -> Result<()> {
    installed()?;
    let user = security::account(user)?;
    ensure!(
        !fs::read_to_string("/etc/pam.d/sddm")?.contains(BEGIN),
        "Calibrate before enabling the SDDM fingerprint branch"
    );
    active()?;
    prompt("Clear the sensor, then press Enter to capture the background.")?;
    let background = image::decode(&super::capture::capture(&super::private_directory())?)?;
    let _ = gxfp_backends::chicago::Chicago::new(&background)?;
    let mut config = Config {
        enabled: false,
        user: user.name,
        uid: user.uid.as_raw(),
    };
    save("config.json", &config)?;
    save("background.json", &background)?;
    remove_report()?;
    config.enabled = true;
    save("config.json", &config)?;
    println!("Calibration stored privately. Enroll with fprintd-enroll, then run check.");
    Ok(())
}
pub(crate) fn check() -> Result<()> {
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
        let pam = super::validation::bounded(
            Command::new(Path::new(ROOT).join("pam_probe"))
                .arg(&c.user)
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("LANG", "C.UTF-8"),
            Duration::from_secs(30),
        )?;
        ensure!(
            pam.code()
                == Some(if expected {
                    0
                } else if label == "empty" {
                    9
                } else {
                    // pam_fprintd exhausts max-tries=1 after verify-no-match.
                    11
                }),
            "Unexpected PAM result for {label}"
        );
        results.push(json!({"case":label,"accepted":expected}));
        println!("Expected standard PAM result observed.");
    }
    ensure!(
        initial == fingerprint()?,
        "Runtime changed during validation"
    );
    save(
        "live-validation.json",
        &json!({"fingerprint":initial,"results":results}),
    )?;
    println!("Five standard PAM checks passed. Local commissioning evidence stored privately.");
    Ok(())
}
pub(crate) fn enable() -> Result<()> {
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
    let updated = enable_text(&original, &c.user, &pam_module()?)?;
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
pub(crate) fn disable() -> Result<()> {
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
        let enabled = enable_text(
            ORIGINAL,
            "testuser",
            Path::new("/usr/lib/security/pam_fprintd.so"),
        )
        .unwrap();
        assert!(enabled.contains("pam_fprintd.so max-tries=1 timeout=20\n"));
        assert_eq!(
            disable_text(&(enabled + "# later edit\n")).unwrap(),
            ORIGINAL.to_owned() + "# later edit\n"
        );
        for module in [
            Path::new("/usr/lib/security/pam_fprintd.so").to_owned(),
            Path::new(ROOT).join("pam_fprintd.so"),
        ] {
            let directive = format!("{} max-tries=1 timeout=20\n", module.display());
            assert!(test_service(&module).contains(&directive));
            assert!(
                enable_text(ORIGINAL, "testuser", &module)
                    .unwrap()
                    .contains(&directive)
            );
        }
    }
    #[test]
    fn rejects_injection_and_ambiguous_layout() {
        for module in ["/tmp/pam_fprintd.so", "/usr/lib/security/pam_permit.so"] {
            assert!(enable_text(ORIGINAL, "testuser", Path::new(module)).is_err());
        }
        for user in [
            "root",
            "test\nauth sufficient pam_permit.so",
            "bad name",
            "1user",
            "a$b",
        ] {
            assert!(
                enable_text(
                    ORIGINAL,
                    user,
                    Path::new("/usr/lib/security/pam_fprintd.so")
                )
                .is_err()
            );
        }
        for text in [
            "auth include common-auth\n",
            "auth required pam_deny.so\nauth include system-login\n",
        ] {
            assert!(
                enable_text(
                    text,
                    "testuser",
                    Path::new("/usr/lib/security/pam_fprintd.so")
                )
                .is_err()
            );
        }
        let enabled = enable_text(
            ORIGINAL,
            "testuser",
            Path::new("/usr/lib/security/pam_fprintd.so"),
        )
        .unwrap();
        assert!(
            enable_text(
                &enabled,
                "testuser",
                Path::new("/usr/lib/security/pam_fprintd.so")
            )
            .is_err()
        );
        assert!(disable_text(&enabled.replace(END, "")).is_err());
        assert_eq!(disable_text(ORIGINAL).unwrap(), ORIGINAL);
    }
}
