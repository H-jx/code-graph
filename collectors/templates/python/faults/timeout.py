"""故障脚本：超时（父进程与子进程都不退出）。"""

from __future__ import annotations

import subprocess
import sys
import time

child = subprocess.Popen(
    [sys.executable, "-c", "import time; time.sleep(600)"],
    stdout=subprocess.DEVNULL,
    stderr=subprocess.DEVNULL,
)
print("fault: sleeping (grandchild alive)", file=sys.stderr)
time.sleep(600)
