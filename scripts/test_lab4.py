"""Actions-only integration: real timer scheduling and UART interrupt input."""
import os
import selectors
import subprocess
import time
from pathlib import Path

command = ["qemu-system-riscv64", "-machine", "virt", "-nographic",
           "-bios", "default", "-global", "virtio-mmio.force-legacy=true",
           "-drive", "file=disk.img,format=raw,id=sfs", "-device", "virtio-blk-device,drive=sfs",
           "-kernel", "os/target/riscv64imac-unknown-none-elf/debug/kernel.bin"]
process = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
selector = selectors.DefaultSelector()
selector.register(process.stdout, selectors.EVENT_READ)
output = ""
def wait_for(text, count=1, timeout=45):
    global output
    deadline = time.monotonic() + timeout
    while output.count(text) < count:
        if time.monotonic() > deadline:
            raise AssertionError(f"timeout waiting for {text!r} x{count}")
        for key, _ in selector.select(0.2):
            chunk = os.read(key.fd, 65536)
            if not chunk:
                raise AssertionError(f"QEMU exited while waiting for {text!r}")
            output += chunk.decode(errors="replace")
    assert "Unresolved interrupt" not in output
    assert "terminated:" not in output

def send(data):
    process.stdin.write(data)
    process.stdin.flush()
try:
    wait_for("KEYBOARD READY")
    assert "Stride policy tests passed" in output
    assert "STRIDE VERIFIED" in output
    send(b"F")
    wait_for("FORK parent=")
    wait_for("FORK CHILD RUNNING")
    wait_for("FORK ISOLATION VERIFIED")
    for count in range(1, 4):
        send(b"\x03")
        wait_for("Ctrl+C terminated thread", count)
    wait_for("all threads terminated, shutting down")
    assert process.wait(timeout=10) == 0
    assert "assertion failed" not in output
    print("LAB4 SUCCESS")
finally:
    if process.poll() is None:
        process.kill()
        process.wait()
    output += process.stdout.read().decode(errors="replace")
    Path("qemu.log").write_text(output)
    print(output)
