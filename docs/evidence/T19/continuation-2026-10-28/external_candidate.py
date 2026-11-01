"""Run the generated artifact via the public Core-only external-candidate protocol."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--business", default="business-01")
    parser.add_argument("--attempt", default="external-core-only-final")
    args = parser.parse_args()
    business = HERE / args.business
    acceptance = json.loads((business / "acceptance-report.json").read_text(encoding="utf-8"))
    assert acceptance["passed"]
    result_path = business / "citation-repair.stdout.json"
    if not result_path.exists():
        result_path = business / "repair.stdout.json"
    if not result_path.exists():
        result_path = business / "resume.stdout.json"
    if not result_path.exists():
        result_path = business / "real-build.stdout.json"
    run = json.loads(result_path.read_text(encoding="utf-8"))
    config = business / "project/.project-knowledge/project.json"
    value = json.loads(config.read_text(encoding="utf-8"))
    revision = config.parent / "work/agent" / run["runId"] / f"revision-{run['revision']:03}"
    out = HERE / args.attempt
    out.mkdir()
    artifact = out / "artifact"
    shutil.copytree(revision, artifact)
    build = subprocess.run(["cargo", "build", "-p", "project-knowledge-cli", "--no-default-features", "--locked"],
                           cwd=ROOT, capture_output=True, timeout=600)
    (out / "build.log").write_bytes(build.stdout + build.stderr)
    assert build.returncode == 0
    binary = ROOT / "target/debug" / ("project-knowledge.exe" if os.name == "nt" else "project-knowledge")
    frozen = ROOT / "target/verified-cli" / sha(binary) / binary.name
    frozen.parent.mkdir(parents=True, exist_ok=True)
    if not frozen.exists():
        shutil.copy2(binary, frozen)
    project = out / "project"
    dest = project / ".project-knowledge/project.json"
    dest.parent.mkdir(parents=True)
    shutil.copy2(business / "project/rules.md", project / "rules.md")
    if (business / "project/flow.ts").exists():
        shutil.copy2(business / "project/flow.ts", project / "flow.ts")
    value.pop("agent")
    dest.write_text(json.dumps(value, ensure_ascii=False, indent=2), encoding="utf-8")
    # No dotenv or provider settings are read by these commands. The binary has
    # no Rig dependency; the artifact remains only files plus JSON contracts.
    env = {k: v for k, v in os.environ.items() if not k.startswith("PROJECT_KNOWLEDGE_")}
    records = []

    def command(name, *args, expected=(0,)):
        process = subprocess.run([str(frozen), *map(str, args)], cwd=ROOT, env=env, capture_output=True, timeout=120)
        (out / (name + ".stdout.json")).write_bytes(process.stdout)
        (out / (name + ".stderr.log")).write_bytes(process.stderr)
        records.append(dict(command=[str(frozen), *map(str, args)], exitCode=process.returncode))
        assert process.returncode in expected, (name, process.returncode)
        return json.loads(process.stdout)

    disabled = command("agent-disabled", "agent", "build", "--config", dest, expected=(1,))
    assert disabled["error"]["code"] == "NOT_IMPLEMENTED", disabled
    command("index", "index", "--config", dest, "--base", expected=(0, 2))
    candidate = command("submit", "plugin", "--config", dest, "submit", "--path", artifact)
    command("prepare", "plugin", "--config", dest, "prepare", "--candidate", candidate["id"])
    command("validate", "plugin", "--config", dest, "validate", "--candidate", candidate["id"])
    activation = command("activate", "plugin", "--config", dest, "activate", "--candidate", candidate["id"])
    repeat = command("activate-repeat", "plugin", "--config", dest, "activate", "--candidate", candidate["id"])
    assert repeat["alreadyCommitted"]
    with sqlite3.connect(dest.parent.joinpath("index.sqlite").resolve().as_uri() + "?mode=ro", uri=True) as db:
        states = sorted(row[0] for row in db.execute("""SELECT json_extract(e.identity_json,'$[#-1]')
            FROM entities e JOIN entity_assertions a ON a.entity_id=e.id
            JOIN current_batches c ON c.batch_id=a.batch_id WHERE e.kind='workflowState'"""))
    assert states == sorted(acceptance["oracle"]["states"])
    assert not (dest.parent / "work/agent").exists()
    dependency = subprocess.run(["cargo", "tree", "-p", "project-knowledge-cli", "--no-default-features", "--locked"], cwd=ROOT, capture_output=True, timeout=30)
    (out / "dependency-tree.log").write_bytes(dependency.stdout + dependency.stderr)
    assert dependency.returncode == 0 and b"rig v" not in dependency.stdout
    report = dict(passed=True, cli=str(frozen), cliSha256=sha(frozen), coreOnly=True, modelRequests=0,
                  sourceArtifactHashes={p.relative_to(artifact).as_posix(): sha(p) for p in artifact.rglob("*") if p.is_file()},
                  candidate=candidate, activation=activation, repeat=repeat, independentStates=states, commands=records)
    (out / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print("Core-only external submit/prepare/validate/activate and repeat activation passed", flush=True)


if __name__ == "__main__":
    main()
