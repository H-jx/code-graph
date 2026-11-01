"""Reproduce T20 gates without models; preserve each command's output and exit code."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import sys
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def save(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

def pnpm(script):
    return ["powershell", "-NoProfile", "-Command", f"pnpm run {script}"] if os.name == "nt" else ["pnpm", "run", script]

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--group", choices=["build", "sdk", "rust", "static", "finalize", "all"], default="all")
    args = parser.parse_args()
    env = dict(os.environ, PK_SDK_EVIDENCE_DIR=str(HERE), PYTHONIOENCODING="utf-8")
    groups = {
        "build": [
            ("build-native", pnpm("build:native")),
            ("build-sdk", pnpm("build:sdk")),
            ("build-cli", ["cargo", "build", "-p", "project-knowledge-cli", "--no-default-features", "--locked"]),
        ],
        "sdk": [("sdk-tests", pnpm("test:sdk"))],
        "rust": [
            ("core-tests", ["cargo", "test", "-p", "project-knowledge-core", "--tests", "--locked", "-j", "1", "--", "--test-threads=1"]),
            ("native-tests", ["cargo", "test", "-p", "project-knowledge-native", "--lib", "--locked", "-j", "1"]),
        ],
        "static": [
            ("clippy", ["cargo", "clippy", "--workspace", "--all-targets", "--locked", "-j", "1", "--", "-D", "warnings"]),
            ("fmt", ["cargo", "fmt", "--all", "--", "--check"]),
            ("contracts", pnpm("check:contracts")), ("typecheck", pnpm("typecheck")),
            ("core-deps", ["node", "scripts/verify-core-dependencies.mjs"]),
            ("native-deps", ["cargo", "tree", "-p", "project-knowledge-native", "--locked", "--prefix", "none"]),
            ("truth", [sys.executable, "scripts/verify-truth.py"]),
            ("plan", ["node", "scripts/verify-plan.mjs"]),
            ("source-audit", ["node", "scripts/audit-source.mjs", "--output", str(HERE / "source-audit.json")]),
            ("diff", ["git", "diff", "--check"]),
        ],
    }
    records_path = HERE / "commands.json"
    records = json.loads(records_path.read_text(encoding="utf-8")) if records_path.exists() else []
    selected = groups if args.group == "all" else {args.group: groups.get(args.group, [])}
    for group, commands in selected.items():
        for name, command in commands:
            attempt = sum(r["name"] == name for r in records) + 1
            log = HERE / f"{name}-{attempt:02}.log"
            print("Running " + " ".join(command), flush=True)
            started = time.monotonic()
            try:
                process = subprocess.run(command, cwd=ROOT, env=env, capture_output=True, timeout=900)
                output, code = process.stdout + process.stderr, process.returncode
            except subprocess.TimeoutExpired as error:
                output, code = (error.stdout or b"") + (error.stderr or b"") + b"\nTIMEOUT after 900s; not passed\n", -1
            log.write_bytes(output)
            record = dict(name=name, group=group, command=command, cwd=str(ROOT), exitCode=code, elapsedSeconds=round(time.monotonic()-started, 3), log=log.name, logSha256=sha(log))
            records.append(record)
            save(records_path, records)
            print(f"{name}: exit={code}, log={log.name}", flush=True)
            if code:
                print(output[-6000:].decode("utf-8", errors="replace"), flush=True)
                return 1
    if args.group not in ("all", "finalize"):
        return 0
    latest = {record["name"]: record for record in records}
    for commands in groups.values():
        for name, _ in commands:
            record = latest[name]
            assert record["exitCode"] == 0 and sha(HERE / record["log"]) == record["logSha256"], name
    deps = (HERE / latest["native-deps"]["log"]).read_text(encoding="utf-8")
    names = {line.split()[0] for line in deps.splitlines() if line.strip()}
    assert not names.intersection({"rig", "rig-core", "project-knowledge-agent", "better-sqlite3", "libsql"})
    for name in ("cli-sdk-golden.json", "event-loop.json", "plugin-lifecycle.json", "readonly-sql.json"):
        assert json.loads((HERE / name).read_text(encoding="utf-8"))["passed"], name
    inventory = subprocess.check_output(["rg", "--files", "crates", "bindings/node", "packages/sdk/src", "packages/sdk/test", "xtask/src"], cwd=ROOT, encoding="utf-8").splitlines()
    sources = [ROOT/name for name in inventory if Path(name).suffix in (".rs", ".ts", ".mjs")]
    sources += [ROOT/p for p in ("Cargo.toml", "Cargo.lock", "bindings/node/package.json", "packages/sdk/package.json", "docs/schema/contract.schema.json", "scripts/verify-contract-types.mjs", ".github/workflows/ci.yml", "plan/NEW-PROJECT-LONG-TASK.md", "docs/execution-log.md", "docs/sdk-usage.md", "docs/evidence/T20/README.md", "docs/evidence/T20/verify.py")]
    native_files = sorted((ROOT/"bindings/node").glob("*.node"))
    assert native_files, "missing built native artifact"
    binaries = [*native_files, ROOT/"bindings/node/index.js", ROOT/"bindings/node/index.d.ts", ROOT/"packages/sdk/dist/client.js", ROOT/"packages/sdk/dist/index.js", ROOT/"target/debug"/("project-knowledge.exe" if os.name == "nt" else "project-knowledge")]
    tools = {}
    for name, command in {"rustc":["rustc","--version"], "node":["node","--version"], "python":[sys.executable,"--version"]}.items():
        tools[name] = subprocess.check_output(command, cwd=ROOT).decode().strip()
    report = dict(task="T20", passed=True, platform=platform.platform(), timezone="Asia/Shanghai", modelRequests=0, businessDatabaseConnections=0, commands=list(latest.values()), sourceHashes={p.relative_to(ROOT).as_posix():sha(p) for p in sorted(set(sources))}, binaryHashes={p.relative_to(ROOT).as_posix():sha(p) for p in binaries}, tools=tools, evidenceHashes={p.name:sha(p) for p in HERE.glob("*.json") if p.name not in ("verification-report.json", "integrity-report.json")}, limitations=["Windows x64 only", "Full CLI/SDK/MCP business matrix remains T22", "Four-platform packaging remains T23"])
    save(HERE/"verification-report.json", report)
    for section in ("sourceHashes", "binaryHashes"):
        for name, digest in report[section].items():
            assert sha(ROOT/name) == digest, name
    for name, digest in report["evidenceHashes"].items():
        assert sha(HERE/name) == digest, name
    audit = json.loads((HERE/"source-audit.json").read_text(encoding="utf-8"))
    assert not audit["zeroByteSources"]
    for name, digest in audit["fileHashes"].items():
        assert sha(ROOT/name) == digest, name
    counts = {}
    for kind, name in (("core", "core-tests"), ("native", "native-tests"), ("sdk", "sdk-tests")):
        output = (HERE/latest[name]["log"]).read_text(encoding="utf-8")
        pattern = r"(?:ℹ |# )pass (\d+)" if kind == "sdk" else r"test result: ok\. (\d+) passed;"
        matches = re.findall(pattern, output)
        assert matches, name
        counts[kind] = sum(map(int, matches))
    counts["total"] = sum(counts.values())
    save(HERE/"integrity-report.json", dict(task="T20", passed=True, verificationReportSha256=sha(HERE/"verification-report.json"), commands=len(report["commands"]), sourceHashesChecked=len(report["sourceHashes"]), artifactHashesChecked=len(report["binaryHashes"]), sourceAuditChecked=audit["filesScanned"], zeroByteSources=0, tests=counts))
    print("T20 verification complete", flush=True)
    return 0

if __name__ == "__main__":
    sys.exit(main())
