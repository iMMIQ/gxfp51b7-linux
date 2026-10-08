// SPDX-License-Identifier: LGPL-3.0-or-later
//! Root-only worker for libfprint. Events and a bounded private print travel
//! through inherited pipes; device images stay in memory.
use anyhow::{Result, ensure};
use gxfp_backends::chicago::{Chicago, Evidence, Print};
use gxfp_core::image;
use std::{
    io::{self, Read, Write},
    time::{Duration, Instant},
};
const MAX_PRINT: usize = 2 * 1024 * 1024;
fn event(message: &str) -> Result<()> {
    writeln!(io::stdout(), "{message}")?;
    io::stdout().flush()?;
    Ok(())
}
fn read_print_from(mut input: impl Read) -> Result<Print> {
    let mut header = [0; 4];
    input.read_exact(&mut header)?;
    let length = u32::from_le_bytes(header) as usize;
    ensure!(
        length > 0 && length <= MAX_PRINT,
        "Bounded Chicago print required"
    );
    let mut bytes = vec![0; length];
    input.read_exact(&mut bytes)?;
    let print: Print = serde_json::from_slice(&bytes)?;
    ensure!(print.version == 1, "Unsupported print version");
    Ok(print)
}
fn capture() -> Result<Vec<u16>> {
    image::decode(&super::capture::capture(&super::private_directory())?)
}
fn presence(raw: &[u16], background: &[u16]) -> Result<bool> {
    let (drop, contrast) = image::quality(raw, background)?;
    Ok(drop >= 900. && contrast >= 40.)
}
fn wait_lift(background: &[u16]) -> Result<()> {
    event("lift")?;
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        let raw = capture()?;
        let (drop, contrast) = image::quality(&raw, background)?;
        if drop < 300. && contrast < 20. {
            return Ok(());
        }
    }
    anyhow::bail!("Lift deadline reached")
}
fn obtain(background: &[u16], deadline: Instant) -> Result<Vec<u16>> {
    event("waiting")?;
    while Instant::now() < deadline {
        // NEEDED remains set throughout acquisition; clients may prompt to hold.
        let raw = capture()?;
        if presence(&raw, background)? {
            event("captured")?;
            return Ok(raw);
        }
    }
    anyhow::bail!("Contact deadline reached")
}
fn retry(e: &Evidence) -> Result<()> {
    event(&format!("retry {}", e.position_reject))
}
pub(crate) fn run(user: &str, enroll: bool) -> Result<()> {
    super::security::root()?;
    let config = super::security::config()?;
    super::security::verify_account(user, &config)?;
    if enroll {
        // The commissioned background gives a conservative finger-off gate.
        let previous = super::background()?;
        let background = previous.as_slice();
        wait_lift(background)?;
        let refreshed = capture()?;
        let (drop, contrast) = image::quality(&refreshed, background)?;
        ensure!(
            drop < 300. && contrast < 20.,
            "Keep the sensor clear for background calibration"
        );
        let mut chicago = Chicago::new(&refreshed)?;
        let overall = Instant::now() + Duration::from_secs(600);
        let mut completed = 0;
        for _ in 0..50 {
            ensure!(Instant::now() < overall, "Enrollment deadline reached");
            let raw = obtain(&refreshed, Instant::now() + Duration::from_secs(30))?;
            let evidence = chicago.enroll(&raw)?;
            if evidence.reject != 0 {
                event("retry 0")?;
            } else if evidence.accepted == 0 {
                retry(&evidence)?;
            } else {
                completed = evidence.count;
                event(&format!("progress {completed}"))?;
            }
            if completed >= 12 {
                let bytes = serde_json::to_vec(&chicago.print()?)?;
                ensure!(bytes.len() <= MAX_PRINT, "Oversized Chicago print");
                event(&format!("print {}", bytes.len()))?;
                io::stdout().write_all(&bytes)?;
                io::stdout().flush()?;
                return Ok(());
            }
            wait_lift(&refreshed)?;
        }
        anyhow::bail!("Enrollment attempt limit reached")
    } else {
        let print = read_print_from(io::stdin())?;
        let mut chicago = Chicago::from_print(&print)?;
        let deadline = Instant::now() + Duration::from_secs(12);
        while Instant::now() < deadline {
            let raw = obtain(&print.background, deadline)?;
            let evidence = chicago.verify(&raw)?;
            if evidence.reject != 0 {
                event("retry 0")?;
                wait_lift(&print.background)?;
                continue;
            }
            // One quality-accepted capture makes one decision. Scores do not
            // trigger further captures or modify the enrolled template.
            event(if evidence.accepted != 0 {
                "match 1"
            } else {
                "match 0"
            })?;
            return Ok(());
        }
        anyhow::bail!("Verification deadline reached")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_print_transport_rejects_truncation_and_oversize() {
        for length in [0u32, MAX_PRINT as u32 + 1, u32::MAX] {
            assert!(read_print_from(length.to_le_bytes().as_slice()).is_err());
        }
        assert!(read_print_from(&[1u8, 0][..]).is_err());
        let mut short = 16u32.to_le_bytes().to_vec();
        short.extend_from_slice(b"{}");
        assert!(read_print_from(short.as_slice()).is_err());
        let body = br#"{"version":2,"background":[],"calibration":[],"gallery":[]}"#;
        let mut future = (body.len() as u32).to_le_bytes().to_vec();
        future.extend_from_slice(body);
        assert!(read_print_from(future.as_slice()).is_err());
    }
}
