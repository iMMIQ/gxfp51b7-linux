# SPDX-License-Identifier: LGPL-3.0-or-later
#!/usr/bin/python
"""Runs as the dedicated unprivileged VM service user."""
import os

os.execv('/usr/bin/qemu-system-x86_64', [
    'qemu-system-x86_64', '-name', 'gxfp51b7-runtime', '-no-user-config',
    '-enable-kvm', '-cpu', 'host,+sgx,+sgx-tokenkey,-sgxlc',
    '-machine', 'q35,sgx-epc.0.memdev=epc,sgx-epc.0.node=0',
    '-object', 'memory-backend-epc,id=epc,size=48M,prealloc=on',
    '-m', '1536', '-smp', '2',
    '-drive', 'file=/var/lib/gxfp51b7-vm/guest.qcow2,if=virtio,format=qcow2',
    '-netdev', 'user,id=n1,restrict=on,hostfwd=tcp:127.0.0.1:2228-:22',
    '-device', 'virtio-net-pci,netdev=n1',
    '-virtfs', 'local,path=/usr/local/share/gxfp51b7-guest,mount_tag=research,security_model=none,readonly=on',
    '-display', 'none', '-serial', 'file:/var/lib/gxfp51b7-vm/serial.log',
    '-monitor', 'none',
])
