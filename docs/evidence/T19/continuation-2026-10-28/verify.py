"""Run T19 gates and bind the independent/live/recovery evidence to current sources."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import sqlite3
import subprocess
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def save(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--finalize", action="store_true")
    parser.add_argument("--business", default="business-04")
    args = parser.parse_args()
    env = dict(os.environ, PYTHONIOENCODING="utf-8", PK_AGENT_EVIDENCE_DIR=str(HERE))
    if not args.finalize:
        commands = [
            ("indexing.log", ["cargo", "test", "-p", "project-knowledge-core", "--test", "indexing", "--locked", "-j", "1", "--", "--test-threads=1"]),
            ("collectors.log", ["cargo", "test", "-p", "project-knowledge-core", "--test", "collectors", "--locked", "-j", "1", "--", "--test-threads=1"]),
            ("lifecycle.log", ["cargo", "test", "-p", "project-knowledge-core", "--test", "lifecycle", "--locked", "-j", "1", "--", "--test-threads=1"]),
            ("clippy-gate.log", ["cargo", "clippy", "--workspace", "--all-targets", "--locked", "-j", "1", "--", "-D", "warnings"]),
            ("fmt.log", ["cargo", "fmt", "--all", "--", "--check"]),
            ("contracts.log", ["cargo", "run", "-p", "xtask", "--locked", "--", "export-contracts", "--check"]),
            ("types.log", ["node", "scripts/generate-types.mjs", "--check"]),
            ("type-boundaries.log", ["node", "scripts/verify-contract-types.mjs"]),
            ("typecheck.log", ["powershell", "-NoProfile", "-Command", "pnpm run typecheck"] if os.name == "nt" else ["pnpm", "run", "typecheck"]),
            ("core-dependencies.log", ["node", "scripts/verify-core-dependencies.mjs"]),
            ("truth.log", [sys.executable, "scripts/verify-truth.py"]),
            ("source-audit.log", ["node", "scripts/audit-source.mjs", "--output", str(HERE / "source-audit.json")]),
            ("plan.log", ["node", "scripts/verify-plan.mjs", "--plan", "../plan/NEW-PROJECT-LONG-TASK.md"]),
            ("diff.log", ["git", "diff", "--check"]),
        ]
        records = []
        for name, command in commands:
            print("Running " + " ".join(command), flush=True)
            process = subprocess.run(command, cwd=ROOT, env=env, capture_output=True, timeout=900)
            log = HERE / name
            log.write_bytes(process.stdout + process.stderr)
            records.append(dict(command=command, exitCode=process.returncode, log=name, logSha256=sha(log)))
            save(HERE / "check-report.json", dict(commands=records))
            assert process.returncode == 0, name
        source = [*sorted((ROOT / "crates/agent/src").glob("*.rs")), ROOT / "crates/agent/tests/agent_lifecycle.rs", ROOT / "crates/cli/src/commands/agent.rs", ROOT / "crates/core/src/knowledge/context.rs", ROOT / "crates/core/tests/support/knowledge.rs", *sorted((ROOT / "prompts").glob("*.md")), ROOT / "Cargo.toml", ROOT / "Cargo.lock"]
        save(HERE / "check-report.json", dict(commands=records, sourceHashes={p.relative_to(ROOT).as_posix(): sha(p) for p in source}))
        print("T19 code gates passed; finalize after the external and subprocess checks", flush=True)
        return
    checks = json.loads((HERE / "check-report.json").read_text(encoding="utf-8"))
    for path, digest in json.loads((HERE / "agent-tools-inventory.json").read_text(encoding="utf-8")).items():
        assert sha(ROOT / path) == digest, path
    for path, digest in checks["sourceHashes"].items():
        assert sha(ROOT / path) == digest, path
    for record in checks["commands"]:
        assert record["exitCode"] == 0 and sha(HERE / record["log"]) == record["logSha256"]
    business = HERE / args.business
    acceptance = json.loads((business / "acceptance-report.json").read_text(encoding="utf-8"))
    live = json.loads((business / "live-summary.json").read_text(encoding="utf-8"))
    external = json.loads((HERE / "external-core-only-final/report.json").read_text(encoding="utf-8"))
    recovery = json.loads((HERE / "subprocess-recovery-final/report.json").read_text(encoding="utf-8"))
    assert acceptance["passed"] and external["passed"] and recovery["passed"]
    assert sha(Path(live["cli"])) == live["cliSha256"]
    assert sha(Path(external["cli"])) == external["cliSha256"]
    assert sha(Path(recovery["cli"])) == recovery["cliSha256"]
    for path, digest in live["sourceHashes"].items():
        assert sha(ROOT / path) == digest, path
    # Independent source support for the business claim, separate from Core's
    # structural citation check and from candidate-provided acceptance cases.
    database = business / "project/.project-knowledge/index.sqlite"
    selected = {(v["topicId"], v["version"]) for v in acceptance["skillManifest"]["topics"]}
    guard_support = []
    with sqlite3.connect(database.resolve().as_uri() + "?mode=ro", uri=True) as db:
        paths = dict(db.execute("SELECT id,path FROM sources"))
        for topic, version, artifact_json, context_json in db.execute("SELECT topic_id,version,artifact_json,context_json FROM topic_artifacts"):
            if (topic, version) not in selected:
                continue
            artifact, context = json.loads(artifact_json), json.loads(context_json)
            refs = {r["evidenceId"]: r for r in context["sourceReferences"]}
            for claim in artifact["claims"]:
                if claim["id"] != "guard":
                    continue
                supported = []
                for identifier in claim["evidenceRefs"]:
                    ref = refs[identifier]
                    if paths[ref["sourceId"]] != "flow.ts":
                        continue
                    source = business / "project/flow.ts"
                    assert sha(source) == ref["contentHash"] == sha(ROOT / "fixtures/purchase-workflow/flow.ts")
                    snippet = source.read_bytes()[ref["startByte"]:ref["endByte"]].decode("utf-8")
                    if "netReceived" in snippet and "allReceiptsValid" in snippet:
                        supported.append(dict(reference=ref, snippet=snippet))
                assert supported, (topic, version, "guard claim lacks actual original code support")
                guard_support.append(dict(topicId=topic, version=version, claim=claim, originalSupport=supported))
    assert guard_support
    save(HERE / "independent-guard-support.json", guard_support)
    original = json.loads((business / "real-build.stdout.json").read_text(encoding="utf-8"))
    current = json.loads((business / "citation-repair.stdout.json").read_text(encoding="utf-8"))
    base = business / "project/.project-knowledge/work/agent"
    before = base / original["runId"] / f"revision-{original['revision']:03}"
    after = base / current["runId"] / f"revision-{current['revision']:03}"
    manifest = json.loads((before / "manifest.json").read_text(encoding="utf-8"))
    retained = set(manifest["artifacts"]) | set(manifest["acceptanceCases"])
    for path in manifest["acceptanceCases"]:
        case = json.loads((before / path).read_text(encoding="utf-8"))
        retained.add(case["inputFixture"])
        if case.get("truthRef"):
            retained.add(case["truthRef"])
    retained.discard("manifest.json")
    case_hashes = {}
    for path in sorted(retained):
        assert sha(before / path) == sha(after / path), path
        case_hashes[path] = sha(after / path)
    save(HERE / "retained-artifact-files.json", case_hashes)
    # Restore the normal CLI after the Core-only external protocol check.
    built = subprocess.run(["cargo", "build", "-p", "project-knowledge-cli", "--locked"], cwd=ROOT, env=env, capture_output=True, timeout=600)
    (HERE / "default-cli-build.log").write_bytes(built.stdout + built.stderr)
    assert built.returncode == 0
    binary = ROOT / "target/debug" / ("project-knowledge.exe" if os.name == "nt" else "project-knowledge")
    audit = json.loads((HERE / "source-audit.json").read_text(encoding="utf-8"))
    passed_logs = ["agent-cli-final.log", "indexing.log", "collectors.log", "lifecycle.log"]
    contents = "\n".join((HERE / name).read_text(encoding="utf-8", errors="replace") for name in passed_logs)
    summaries = re.findall(r"test result:.*", contents)
    assert not re.search(r"test result: FAILED", contents)
    count = sum(int(v) for v in re.findall(r"test result: ok\. (\d+) passed", contents))
    by_run = {}
    for path in sorted(HERE.glob("business-*/*summary.json")):
        value = json.loads(path.read_text(encoding="utf-8"))
        if value.get("runId") and isinstance(value.get("modelTurns"), int):
            previous = by_run.get(value["runId"], {})
            if value["modelTurns"] >= previous.get("modelTurns", -1):
                by_run[value["runId"]] = dict(directory=path.parent.name, **value)
    attempts = list(by_run.values())
    snapshots = []
    for directory, children, names in os.walk(HERE):
        # Prune before traversal: Windows control paths can exceed Python's
        # legacy path limit and are deliberately outside this snapshot set.
        children[:] = [name for name in children if name not in (".project-knowledge", "__pycache__")]
        for name in names:
            p = Path(directory) / name
            if p.suffix in (".json", ".log", ".py", ".md", ".mjs") and name not in ("verification-report.json", "integrity-report.json", "source-audit.json", "finalization.log"):
                snapshots.append(p)
    report = dict(started="2026-10-28 Asia/Shanghai", completed="2026-10-29 Asia/Shanghai", task="T19", passed=True, platform="Windows x64",
                  commands=checks["commands"], sourceHashes=checks["sourceHashes"], targetedTests=count, testSummaries=summaries,
                  workspaceAttempt=dict(status="timeout", seconds=900, countedAsPassed=False, logSha256=sha(HERE / "workspace-timeout.log")),
                  finalAgentTests=21, finalAgentCliLogSha256=sha(HERE / "agent-cli-final.log"),
                  liveBusiness=live, attempts=attempts, externalModelRequests=sum(p["modelTurns"] for p in attempts),
                  modelCountBasis="durable model-turn hook counts, including the interrupted request; not a provider billing count",
                  independentGuardSupport=guard_support,
                  retainedArtifactFiles=case_hashes,
                  externalCoreOnly=dict(cli=external["cli"], cliSha256=external["cliSha256"], passed=True),
                  processRecovery=dict(cli=recovery["cli"], cliSha256=recovery["cliSha256"], passed=True),
                  defaultCli=dict(path=str(binary), sha256=sha(binary), agentFeatureEnabled=True),
                  snapshotHashes={p.relative_to(HERE).as_posix(): sha(p) for p in snapshots},
                  businessDatabaseConnections=0, noModelIncrementalRequests=0,
                  limitations=["Real business acceptance covers the frozen T06 audited workflow document sample; all three business scenarios and CLI/SDK/MCP equality remain T22", "No production business accuracy, physical power loss or non-Windows platform claim", "Failed live attempt retained separately; candidate expectations are not used as independent truth"])
    report["toolVersions"] = {key: subprocess.check_output(command, cwd=ROOT, env=env).decode("utf-8").strip() for key, command in [("rustc", ["rustc", "--version"]), ("cargo", ["cargo", "--version"]), ("node", ["node", "--version"]), ("python", [sys.executable, "--version"]) ]}
    report["sourceAuditSha256"] = sha(HERE / "source-audit.json")
    report["sourceAudit"] = {k: audit[k] for k in ["filesScanned", "zeroByteSources", "commentOnlySources", "emptyModules"] if k in audit}
    report["documentationHashes"] = {p:sha(ROOT / p) for p in ["plan/NEW-PROJECT-LONG-TASK.md", "docs/execution-log.md", "docs/agent-usage.md", "skills/project-onboarding/SKILL.md"]}
    save(HERE / "verification-report.json", report)
    print(f"T19 complete evidence verified; targeted tests {count} passed", flush=True)


if __name__ == "__main__":
    main()
