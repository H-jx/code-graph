"""故障脚本：stdout 不是合法 JSON。"""

import sys

print("fault: writing invalid JSON", file=sys.stderr)
sys.stdout.write("this is not json\n")
