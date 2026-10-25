"""采集脚本模板（Python）。

协议（计划 §3.5）：
- stdin 收到一个 UTF-8 JSON 对象（CollectorInput），读取到 EOF 表示输入结束；
- stdout 只能输出一个 JSON 对象（CollectorOutput）；
- 调试日志写 stderr，退出码非零表示整阶段失败。

模板只演示协议骨架：真实插件要按项目调查结论解析源码，并附原始证据。
"""

from __future__ import annotations

import json
import sys
from typing import Any


def read_stdin() -> str:
    try:
        return sys.stdin.read()
    except OSError as error:  # pragma: no cover - 仅在 stdin 不可用时触发
        print(f"cannot read stdin: {error}", file=sys.stderr)
        raise SystemExit(2)


def parse_input() -> dict[str, Any]:
    raw = read_stdin()
    if not raw.strip():
        print("stdin is empty; expected one CollectorInput JSON object", file=sys.stderr)
        raise SystemExit(2)
    try:
        return json.loads(raw)
    except json.JSONDecodeError as error:
        print(f"stdin is not valid JSON: {error}", file=sys.stderr)
        raise SystemExit(2)


def empty_output(payload: dict[str, Any]) -> dict[str, Any]:
    return {
        "protocolVersion": payload["protocolVersion"],
        "runId": payload["runId"],
        "stageId": payload["stageId"],
        "inputSnapshotId": payload["inputSnapshotId"],
        "entities": [],
        "relations": [],
        "evidence": [],
        "dependencies": [],
        "coverage": [],
        "gaps": [],
    }


def main() -> None:
    payload = parse_input()
    output = empty_output(payload)
    print(
        "collector {plugin_id}@{version} stage {stage_id}: {sources} sources, {entities} entities".format(
            plugin_id=payload["pluginId"],
            version=payload["version"],
            stage_id=payload["stageId"],
            sources=len(payload["sources"]),
            entities=len(payload["entities"]),
        ),
        file=sys.stderr,
    )
    sys.stdout.write(json.dumps(output, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
