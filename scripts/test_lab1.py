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
assert "Breakpoint at " in output, "breakpoint handler did not execute"
assert "SUCCESS!" in output, "null-address LoadFault was not handled"
assert "LoadFault at 0x0" in output, "expected terminating LoadFault panic"
assert "Unresolved interrupt" not in output, "unexpected trap"
