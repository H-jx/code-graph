"""Recheck the real-model artifact with a Core-only CLI and an independent JSON oracle."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import subprocess

REPO = Path(__file__).resolve().parents[3]
FIXTURE = REPO / "docs/evidence/T18/live-project"
CONFIG = FIXTURE / ".project-knowledge/project.json"
OUT = Path(__file__).resolve().parent
BASELINE = {"routerId": "app", "mode": "history", "routes": [
    {"path": "/orders", "name": "Orders"}, {"path": "/inventory", "name": "Inventory"}]}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cli", default=str(REPO / "target/agent-verification/core-only.exe"))
    args = parser.parse_args()
    executable = str(Path(args.cli).resolve())
    environment = dict(os.environ)
    for key in list(environment):
        if key.startswith("PROJECT_KNOWLEDGE_"):
            environment.pop(key)
    commands = []

    def cli(*argv):
        process = subprocess.run([executable, *argv], cwd=REPO, env=environment,
                                 capture_output=True, encoding="utf-8", timeout=120)
        record = {"argv": list(argv), "exitCode": process.returncode}
        commands.append(record)
        response = json.loads(process.stdout)
        record["response"] = response
        assert process.returncode == 0, record
        return response

    source = FIXTURE / "routes.json"
    FIXTURE.mkdir(parents=True, exist_ok=True)
    if not CONFIG.exists():
        cli("init", "--root", str(FIXTURE), "--project-id", "live-project")
    if not (CONFIG.parent / "index.sqlite").exists():
        source.write_text(json.dumps(BASELINE, ensure_ascii=False) + "\n", encoding="utf-8")
        cli("index", "--config", str(CONFIG), "--base")
        candidate = cli("plugin", "--config", str(CONFIG), "submit", "--path", str(OUT / "artifact"))
        cli("plugin", "--config", str(CONFIG), "prepare", "--candidate", candidate["id"])
        cli("plugin", "--config", str(CONFIG), "validate", "--candidate", candidate["id"])
        cli("plugin", "--config", str(CONFIG), "activate", "--candidate", candidate["id"])

    def current_routes():
        database = (CONFIG.parent / "index.sqlite").resolve()
        with sqlite3.connect(database.as_uri() + "?mode=ro", uri=True) as connection:
            rows = connection.execute("""SELECT e.id,e.identity_json,b.state,a.batch_id
                FROM entities e JOIN entity_assertions a ON a.entity_id=e.id
                JOIN current_batches c ON c.batch_id=a.batch_id
                JOIN batches b ON b.id=a.batch_id WHERE e.kind='route' ORDER BY e.id""").fetchall()
        assert all(state == "fresh" for _, _, state, _ in rows)
        return [{"entityId": entity_id, "identity": json.loads(identity), "state": state,
                 "batchId": batch_id} for entity_id, identity, state, batch_id in rows]

    def assert_paths(rows, expected):
        assert sorted(row["identity"][-1] for row in rows) == sorted(expected), rows

    assert source.resolve().parent == FIXTURE.resolve()
    original = source.read_bytes()
    runs = CONFIG.parent / "work/agent"
    run_count = len(list(runs.glob("agent_*"))) if runs.exists() else 0
    observations = []
    try:
        assert_paths(current_routes(), ["/orders", "/inventory"])
        changes = [
            ("unicode-add-delete", {"routerId": "app", "mode": "history", "routes": [
                {"path": "/orders", "name": "订单"}, {"path": "/库存", "name": "库存"},
                {"path": "/audit", "name": "Audit"}]}, ["/orders", "/库存", "/audit"]),
            ("empty-output", {"routerId": "app", "mode": "history", "routes": []}, []),
            ("source-deletion", None, []),
        ]
        for label, document, expected in changes:
            if document is None:
                source.unlink()
            else:
                source.write_text(json.dumps(document, ensure_ascii=False) + "\n", encoding="utf-8")
            response = cli("index", "--config", str(CONFIG), "--incremental")
            rows = current_routes()
            assert_paths(rows, expected)
            assert response["executed"] == ["live-project.routes:extract"], response
            observations.append({"case": label, "expectedPaths": expected, "actual": rows,
                                 "sourceHash": hashlib.sha256(source.read_bytes()).hexdigest() if source.exists() else None})
    finally:
        source.write_bytes(original)
        restored = cli("index", "--config", str(CONFIG), "--incremental")
    rows = current_routes()
    assert_paths(rows, ["/orders", "/inventory"])
    after = len(list(runs.glob("agent_*"))) if runs.exists() else 0
    assert after == run_count
    report = {"passed": True, "oracle": "JSON paths independently read by this verifier; candidate expectations are not used",
              "businessTruthClaim": False, "cli": executable, "coreOnly": True,
              "modelRequests": 0, "basis": "Core-only binary has no Agent/Rig; credential variables removed; agent run count unchanged",
              "agentRunsBefore": run_count, "agentRunsAfter": after, "observations": observations,
              "restored": {"response": restored, "routes": rows}, "commands": commands}
    (OUT / "no-model-incremental.json").write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({"passed": True, "cases": len(observations), "modelRequests": 0, "restored": True}))


if __name__ == "__main__":
    main()
