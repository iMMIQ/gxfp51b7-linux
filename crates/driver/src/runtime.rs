// SPDX-License-Identifier: LGPL-3.0-or-later
use anyhow::{Result, bail, ensure};
use std::{
    os::unix::process::CommandExt,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use wait_timeout::ChildExt;

pub(crate) fn start() -> Result<()> {
    let user = nix::unistd::User::from_name("gxfpvm")?
        .ok_or_else(|| anyhow::anyhow!("Missing VM account"))?;
    ensure!(
        !user.uid.is_root() && nix::unistd::Uid::effective() == user.uid,
        "Use the dedicated VM service account"
    );
    let error=Command::new("/usr/bin/qemu-system-x86_64").args([
        "-name","gxfp51b7-runtime","-no-user-config","-enable-kvm","-cpu","host,+sgx,+sgx-tokenkey,-sgxlc",
        "-machine","q35,sgx-epc.0.memdev=epc,sgx-epc.0.node=0","-object","memory-backend-epc,id=epc,size=48M,prealloc=on",
        "-m","1536","-smp","2","-drive","file=/var/lib/gxfp51b7-vm/guest.qcow2,if=virtio,format=qcow2",
        "-netdev","user,id=n1,restrict=on,hostfwd=tcp:127.0.0.1:2228-:22","-device","virtio-net-pci,netdev=n1",
        "-virtfs","local,path=/usr/local/share/gxfp51b7-guest,mount_tag=research,security_model=none,readonly=on",
        "-display","none","-serial","file:/var/lib/gxfp51b7-vm/serial.log","-monitor","none",
    ]).env_clear().env("PATH","/usr/bin:/bin").env("LANG","C.UTF-8").exec();
    Err(error.into())
}
pub(crate) fn ready() -> Result<()> {
    super::security::root()?;
    let private = super::private_directory();
    for name in ["known_hosts", "id_ed25519"] {
        super::security::trusted(&private.join(name))?;
    }
    let deadline = Instant::now() + Duration::from_secs(45);
    while Instant::now() < deadline {
        let mut child=super::capture::ssh(&private,1).arg("test -e /dev/isgx && test -x /home/ubuntu/legacy_load && test -f /mnt/research/sgxs/WBDI_Enclave.signed.sgxs && systemctl is-active --quiet gxfp51b7-guest && echo GXFP_READY")
            .stdin(Stdio::null()).stderr(Stdio::null()).stdout(Stdio::piped()).spawn()?;
        if child.wait_timeout(Duration::from_secs(4))?.is_none() {
            let _ = child.kill();
            let _ = child.wait();
        } else {
            let out = child.wait_with_output()?;
            if out.status.success() && out.stdout == b"GXFP_READY\n" {
                println!("GXFP51B7 isolated runtime ready");
                return Ok(());
            }
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    bail!("Isolated runtime readiness timed out")
}
