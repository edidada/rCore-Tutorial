"""Run only in Actions: check the real trap path, not just compilation."""
import subprocess
from pathlib import Path

command = ["qemu-system-riscv64", "-machine", "virt", "-nographic",
           "-bios", "default", "-kernel",
           "os/target/riscv64imac-unknown-none-elf/debug/kernel.bin"]
try:
    run = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                         timeout=30)
    output = run.stdout.decode(errors="replace")
except subprocess.TimeoutExpired as error:
    output = (error.stdout or b"").decode(errors="replace")
    Path("qemu.log").write_text(output)
    print(output)
    raise SystemExit("QEMU timed out before shutdown")
Path("qemu.log").write_text(output)
print(output)
assert run.returncode == 0, run.returncode
assert "kernel remapped" in output
assert "mapping tests passed" in output
assert "LAB3 SUCCESS" in output
assert "Unresolved interrupt" not in output
