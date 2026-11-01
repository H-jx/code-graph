"""合成宿主 smoke，必须使用准备好的 Python 3.12 venv。"""
import json
import os
import sys
import packaging
from packaging.version import Version

assert sys.version_info[:2] == (3, 12)
assert packaging.__version__ == "25.0"
assert str(Version("1.2.3")) == "1.2.3"
assert not any(os.getenv(key) for key in ("OPENAI_API_KEY", "ANTHROPIC_API_KEY", "PYTHONHOME"))
if sys.platform == "win32":
    assert packaging.__file__.startswith(os.environ["PYTHONPATH"])
else:
    assert not os.getenv("PYTHONPATH")
assert sys.prefix != sys.base_prefix
payload = json.load(sys.stdin)
json.dump({"protocolVersion":"1.0","runId":payload["runId"],"stageId":payload["stageId"],"inputSnapshotId":payload["inputSnapshotId"],"entities":[],"relations":[],"evidence":[],"dependencies":[],"coverage":[],"gaps":[]},sys.stdout)
