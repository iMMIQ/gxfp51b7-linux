// SPDX-License-Identifier: LGPL-3.0-or-later
use anyhow::{Result, bail, ensure};
use gxfp_core::protocol::{self, encode};
use memmap2::{MmapOptions, MmapRaw};
use nix::fcntl::{Flock, FlockArg};
use std::{
    fs::{self, File, OpenOptions},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    ptr,
    time::{Duration, Instant},
};

pub const SIZE: usize = 0x8000;
pub const RX: usize = 0x1000;
pub struct Mailbox {
    map: MmapRaw,
    _lock: Flock<File>,
}
impl Mailbox {
    pub fn open() -> Result<Self> {
        super::security::root()?;
        ensure!(
            fs::read_to_string("/sys/class/dmi/id/sys_vendor")?.trim() == "HUAWEI"
                && fs::read_to_string("/sys/class/dmi/id/product_name")?.trim() == "MACHC-WAX9",
            "Unsupported machine"
        );
        ensure!(
            std::path::Path::new("/sys/bus/acpi/devices/GXFP51B7:00").exists(),
            "GXFP51B7 absent"
        );
        ensure!(
            !std::path::Path::new("/sys/devices/platform/GXFP51B7:00/driver").exists(),
            "Diagnostic driver is bound"
        );
        ensure!(
            fs::read_to_string("/proc/iomem")?.lines().any(|line| {
                line.split_once(':').is_some_and(|(a, b)| {
                    a.trim() == "40200000-40207fff" && b.trim() == "GXFP51B7:00"
                })
            }),
            "Reserved mailbox missing"
        );
        ensure!(
            std::path::Path::new("/proc/acpi/call").exists(),
            "acpi_call unavailable"
        );
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(nix::libc::O_NOFOLLOW)
            .open("/run/lock/gxfp51b7-research.lock")?;
        let m = lock.metadata()?;
        ensure!(
            m.uid() == 0 && m.mode() & 0o022 == 0 && m.is_file(),
            "Unsafe capture lock"
        );
        let lock = Flock::lock(lock, FlockArg::LockExclusiveNonblock).map_err(|(_, e)| e)?;
        let memory = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(nix::libc::O_SYNC)
            .open("/dev/mem")?;
        // Platform/resource checks select this exact reserved EC range.
        // MmapRaw exposes pointers, allowing volatile I/O without Rust slices.
        let map = MmapOptions::new()
            .offset(0x40200000)
            .len(SIZE)
            .map_raw(&memory)?;
        Ok(Self { map, _lock: lock })
    }
    pub fn snapshot(&self, offset: usize, len: usize) -> Result<Vec<u8>> {
        ensure!(
            offset <= SIZE && len <= SIZE - offset,
            "Read outside mailbox"
        );
        // SAFETY: byte-aligned offsets stay within the checked mapping. Volatile
        // reads observe EC updates; packet stability is checked by the caller.
        Ok((offset..offset + len)
            .map(|i| unsafe { ptr::read_volatile(self.map.as_ptr().add(i)) })
            .collect())
    }
    pub fn send(&mut self, packet: &[u8]) -> Result<()> {
        ensure!(packet.len() <= RX, "Transmit outside mailbox");
        for (i, &value) in packet.iter().enumerate() {
            // SAFETY: validated TX bounds and exclusive host lock; device memory
            // has byte access semantics and is not exposed as a Rust slice.
            unsafe { ptr::write_volatile(self.map.as_mut_ptr().add(i), value) };
        }
        self.doorbell()
    }
    fn doorbell(&self) -> Result<()> {
        fs::write(
            "/proc/acpi/call",
            r"\_SB.SPBA._DSM b8ab658cc79449348a8bb961209db59e5 0 2 {}",
        )?;
        let result = fs::read_to_string("/proc/acpi/call")?;
        ensure!(!result.starts_with("Error"), "ACPI doorbell failed");
        Ok(())
    }
    pub fn tls_hello(&mut self) -> Result<()> {
        let before = self.snapshot(RX, 0x1000)?;
        self.send(&encode(0xd0, &[0, 0])?)?;
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            let raw = self.snapshot(RX, 0x1000)?;
            if raw != before
                && let Some(record) = protocol::tls_record(&raw)
                && record.len() >= 48
                && record[0] == 22
                && record[5] == 1
                && raw[..record.len() + 4] != before[..record.len() + 4]
            {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        bail!("TLS ClientHello timed out")
    }
}
