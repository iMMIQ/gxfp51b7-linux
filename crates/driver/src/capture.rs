// SPDX-License-Identifier: LGPL-3.0-or-later
use super::mailbox::{Mailbox, RX, SIZE};
use anyhow::{Result, bail, ensure};
use gxfp_core::protocol::{self, encode};
use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
use std::{
    collections::VecDeque,
    fs,
    io::{Read, Seek, SeekFrom, Write},
    os::fd::AsFd,
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
use wait_timeout::ChildExt;

struct Guest(Child);
impl Drop for Guest {
    fn drop(&mut self) {
        let _ = self.0.stdin.take();
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}
fn exact(p: &mut Child, n: usize, deadline: Instant) -> Result<Vec<u8>> {
    let out = p
        .stdout
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("Missing guest pipe"))?;
    let mut result = vec![0; n];
    let mut got = 0;
    while got < n {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or_else(|| anyhow::anyhow!("Capture timed out"))?;
        let mut descriptors = [PollFd::new(out.as_fd(), PollFlags::POLLIN)];
        ensure!(
            poll(
                &mut descriptors,
                PollTimeout::try_from(remaining.min(Duration::from_secs(12)))?
            )? > 0,
            "Capture timed out"
        );
        match out.read(&mut result[got..]) {
            Ok(0) => bail!("Guest exited unexpectedly"),
            Ok(count) => got += count,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Ok(result)
}
fn respond(p: &mut Child, data: &[u8]) -> Result<()> {
    let input = p
        .stdin
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("Missing guest input"))?;
    input.write_all(data)?;
    input.flush()?;
    Ok(())
}
#[derive(Default)]
struct Records {
    pending: VecDeque<Vec<u8>>,
    previous: Option<Vec<u8>>,
    candidate: Option<(Vec<u8>, Instant)>,
    configured: bool,
}
impl Records {
    fn poll(&mut self, device: &Mailbox) -> Result<()> {
        let raw = device.snapshot(RX, SIZE - RX)?;
        if protocol::decode(&raw) == Some((0x90, &[1, 1][..])) {
            self.configured = true;
        }
        if let Some(record) = protocol::tls_record(&raw) {
            if self.previous.as_deref() == Some(record) {
                return Ok(());
            }
            if record.len() > 512 {
                if self.candidate.as_ref().is_none_or(|(c, _)| c != record) {
                    self.candidate = Some((record.to_vec(), Instant::now()));
                    return Ok(());
                }
                if self.candidate.as_ref().unwrap().1.elapsed() < Duration::from_millis(30) {
                    return Ok(());
                }
            }
            self.pending.push_back(record.to_vec());
            self.previous = Some(record.to_vec());
        }
        Ok(())
    }
}
pub(crate) fn ssh(private: &Path, timeout: u32) -> Command {
    let mut command = Command::new("/usr/bin/ssh");
    command
        .args([
            "-o",
            "BatchMode=yes",
            "-o",
            &format!("ConnectTimeout={timeout}"),
            "-o",
            "StrictHostKeyChecking=yes",
            "-o",
            "IdentitiesOnly=yes",
            "-o",
            "IdentityAgent=none",
            "-o",
            "ForwardAgent=no",
            "-o",
        ])
        .arg(format!(
            "UserKnownHostsFile={}",
            private.join("known_hosts").display()
        ))
        .arg("-i")
        .arg(private.join("id_ed25519"))
        .args(["-p", "2228", "ubuntu@127.0.0.1"])
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LANG", "C.UTF-8");
    command
}
pub(crate) fn capture(private: &Path) -> Result<Vec<u8>> {
    super::security::root()?;
    for name in ["id_ed25519", "known_hosts", "chicago-default-config.bin"] {
        super::security::trusted(&private.join(name))?;
    }
    let sealed = fs::read("/dev/goodix_bios_sealed")?;
    ensure!(
        sealed.len() == 885 && sealed[..4] == [4, 0, 2, 0],
        "Unexpected BIOS sealed container"
    );
    let mut log = tempfile::tempfile()?;
    let mut command = ssh(private, 3);
    command.arg("sudo timeout 12 /home/ubuntu/legacy_load /mnt/research/sgxs/le.signed.sgxs /mnt/research/sgxs/le.production.sigstruct /mnt/research/windows-psw218/white_list_cert.bin /mnt/research/sgxs/WBDI_Enclave.signed.sigstruct /mnt/research/sgxs/WBDI_Enclave.signed.sgxs")
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(log.try_clone()?);
    let deadline = Instant::now() + Duration::from_secs(12);
    let mut guest = Guest(command.spawn()?);
    let p = &mut guest.0;
    respond(p, &sealed)?;
    let mut device = Mailbox::open()?;
    let mut records = Records::default();
    let mut source = None;
    let mut handshake = None;
    let mut finished = false;
    let mut sent = false;
    loop {
        let header: [u8; 12] = exact(p, 12, deadline)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid RPC"))?;
        let (kind, n) = protocol::rpc_header(&header)?;
        match kind {
            2 => {
                device.tls_hello()?;
                records.poll(&device)?;
                respond(p, &0_i64.to_le_bytes())?;
            }
            0 => {
                ensure!(n > 0 && n <= 65536, "Invalid receive length");
                if finished && !sent {
                    records.configured = false;
                    let cfg = fs::read(private.join("chicago-default-config.bin"))?;
                    ensure!(cfg.len() == 256, "Invalid configuration");
                    device.send(&encode(0x90, &cfg)?)?;
                    let until = Instant::now() + Duration::from_millis(300);
                    while !records.configured && Instant::now() < until {
                        records.poll(&device)?;
                        std::thread::sleep(Duration::from_micros(300));
                    }
                    ensure!(records.configured, "Sensor configuration unacknowledged");
                    device.send(&encode(0x20, &[1, 0])?)?;
                    sent = true;
                }
                while records.pending.is_empty() {
                    ensure!(Instant::now() < deadline, "No capture record");
                    records.poll(&device)?;
                    std::thread::sleep(Duration::from_micros(300));
                }
                let record = records.pending.pop_front().unwrap();
                let take = (n as usize).min(record.len());
                if take < record.len() {
                    records.pending.push_front(record[take..].to_vec());
                }
                respond(p, &(take as i64).to_le_bytes())?;
                respond(p, &record[..take])?;
            }
            1 => {
                ensure!(n > 0 && n <= 4092, "Oversized transmit");
                let record = exact(p, n as usize, deadline)?;
                records.poll(&device)?;
                let mut packet = vec![0xb0];
                packet.extend((n as u16).to_le_bytes());
                packet.push(packet.iter().fold(0_u8, |s, &v| s.wrapping_add(v)));
                packet.extend(&record);
                device.send(&packet)?;
                if record[0] == 22 && n == 85 {
                    finished = true;
                }
                let until = Instant::now() + Duration::from_millis(3);
                while Instant::now() < until {
                    records.poll(&device)?;
                    std::thread::sleep(Duration::from_micros(100));
                }
                respond(p, &(n as i64).to_le_bytes())?;
            }
            4 => handshake = Some(n),
            5 => {
                ensure!(
                    n == 10560 && source.is_none(),
                    "Invalid image source length"
                );
                source = Some(exact(p, n as usize, deadline)?);
            }
            3 => {
                ensure!(
                    n == 10573 && handshake == Some(16) && source.is_some(),
                    "Enclave capture failed"
                );
                break;
            }
            _ => bail!("Unexpected guest RPC operation"),
        }
    }
    p.stdin.take();
    let status = p
        .wait_timeout(Duration::from_secs(2))?
        .ok_or_else(|| anyhow::anyhow!("Guest exit timed out"))?;
    log.seek(SeekFrom::Start(0))?;
    let mut messages = Vec::new();
    log.take(65536).read_to_end(&mut messages)?;
    let contains = |needle: &[u8]| messages.windows(needle.len()).any(|part| part == needle);
    ensure!(
        status.success()
            && contains(b"data_from_device_ecall crc check ok")
            && !contains(b"0x7180")
            && !contains(b"crc check failed"),
        "Capture integrity validation failed"
    );
    source.ok_or_else(|| anyhow::anyhow!("Missing source"))
}
