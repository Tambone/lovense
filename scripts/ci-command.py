"""Run a CI command and make its failure readable through GitHub annotations."""

from collections import deque
import os
import shutil
import subprocess
import sys


def main():
    if len(sys.argv) < 2:
        raise SystemExit("Usage: ci-command.py COMMAND [ARGUMENT ...]")
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    tail = deque(maxlen=40)
    command = shutil.which(sys.argv[1]) or sys.argv[1]
    arguments = [command, *sys.argv[2:]]
    # Flutter is a batch launcher on Windows. CI supplies fixed command arguments.
    use_shell = os.name == "nt" and command.lower().endswith((".bat", ".cmd"))
    with subprocess.Popen(
        arguments, shell=use_shell, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        text=True, encoding="utf-8", errors="replace",
    ) as process:
        for line in process.stdout:
            print(line, end="", flush=True)
            tail.append(line.rstrip()[:1000])
        code = process.wait()
    if code:
        message = "\n".join(tail) or f"Command exited with code {code}."
        message = message.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")
        print(f"::error title=CI command failed::{message}", flush=True)
    return code


if __name__ == "__main__":
    sys.exit(main())
