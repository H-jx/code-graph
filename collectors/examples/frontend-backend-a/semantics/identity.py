"""Python 3.12 的独立身份编码实现；仅共享 DTO 导出的槽位规格与协议向量。"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path
import sys
from typing import Any


def canonical(value: Any) -> str:
    if value is None or isinstance(value, (bool, float)):
        raise ValueError("null, booleans and floats are not identity values")
    if isinstance(value, int) and not -(2**63) <= value < 2**64:
        raise ValueError("integer exceeds the protocol's JSON number range")
    if isinstance(value, list):
        return "[" + ",".join(canonical(item) for item in value) + "]"
    if isinstance(value, dict):
        return "{" + ",".join(canonical(key) + ":" + canonical(value[key]) for key in sorted(value)) + "}"
    if isinstance(value, (str, int)):
        return json.dumps(value, ensure_ascii=False, separators=(",", ":"))
    raise ValueError(f"unsupported identity type: {type(value).__name__}")


def stable_id(prefix: str, value: Any) -> str:
    return prefix + hashlib.sha256(canonical(value).encode("utf-8")).hexdigest()


def evaluate(vector: dict[str, Any], specs: dict[str, Any]) -> str:
    match vector["type"]:
        case "entity":
            slots = vector["slots"]
            if vector["kind"] not in specs:
                if not re.fullmatch(r"x\.[A-Za-z0-9_-]+(?:\.[A-Za-z0-9_-]+)*\.[A-Za-z][A-Za-z0-9_-]*", vector["kind"]):
                    raise ValueError("unknown entity kind")
                if not slots or slots[0]["name"] != "scopeId" or not isinstance(slots[0].get("value"), str) or len({s["name"] for s in slots}) != len(slots):
                    raise ValueError("extension identity needs unique named slots beginning with scopeId")
                if any(not s["name"] or set(s) - {"name", "value"} for s in slots):
                    raise ValueError("invalid extension identity slot")
                return stable_id("ent_", [vector["projectId"], vector["kind"], [[s["name"], [s["value"]] if "value" in s else []] for s in slots]])
            components = specs[vector["kind"]]
            if len(components) != len(slots):
                raise ValueError("wrong identity slot count")
            values = [vector["projectId"], vector["kind"]]
            for component, slot in zip(components, slots, strict=True):
                if slot["name"] != component["name"] or set(slot) - {"name", "value"}:
                    raise ValueError("identity slot name or fields do not match the protocol")
                if component["required"]:
                    values.append(slot["value"])
                else:
                    values.append([slot["value"]] if "value" in slot else [])
            return stable_id("ent_", values)
        case "relation":
            return stable_id("rel_", [vector["projectId"], vector["kind"], vector["fromId"], vector["toId"], vector.get("qualifiers", {})])
        case "source":
            return stable_id("src_", [vector["projectId"], vector["repositoryId"], vector["path"]])
        case "evidence":
            return stable_id("ev_", [vector["sourceId"], vector["contentHash"], vector["startByte"], vector["endByte"], vector["role"]])
        case _:
            raise ValueError("unknown vector type")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    root = Path(__file__).resolve().parent.parent
    parser.add_argument("--vectors", type=Path, default=root / "fixtures/protocol/identity-vectors.json")
    args = parser.parse_args()
    if sys.version_info[:2] != (3, 12):
        print("Python 3.12 is required", file=sys.stderr)
        return 2
    specs = {kind["kind"]: kind["identityComponents"] for kind in json.loads((root / "docs/schema/entity-kinds.json").read_text(encoding="utf-8"))["entityKinds"]}
    vectors = json.loads(args.vectors.read_text(encoding="utf-8"))["vectors"]
    results = []
    for vector in vectors:
        try:
            actual = evaluate(vector, specs)
            results.append({"id": vector["id"], "actual": actual, "expected": vector["expectedId"], "ok": actual == vector["expectedId"]})
        except (ValueError, KeyError, TypeError, UnicodeError) as error:
            results.append({"id": vector["id"], "ok": False, "error": str(error)})
    passed = sum(item["ok"] for item in results)
    print(json.dumps({"python": sys.version.split()[0], "total": len(results), "passed": passed, "failed": len(results) - passed, "results": results}, ensure_ascii=False, indent=2))
    return 0 if results and passed == len(results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
