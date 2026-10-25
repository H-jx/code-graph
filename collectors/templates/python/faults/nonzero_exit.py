"""故障脚本：非零退出码。"""

import sys

print("fault: exiting with code 3", file=sys.stderr)
sys.exit(3)
