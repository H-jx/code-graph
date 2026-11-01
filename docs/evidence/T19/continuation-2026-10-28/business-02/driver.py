"""Generate on a frozen T06 business sample; evaluate outside candidate assertions."""
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


def save(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--attempt", default="business-01")
    parser.add_argument("--resume")
    args = parser.parse_args()
    out = HERE / args.attempt
    out.mkdir(parents=True, exist_ok=True)
    project = out / "project"
    config = project / ".project-knowledge/project.json"
    records = []
    build_sources = [ROOT / "crates/agent/src/runner.rs", ROOT / "crates/agent/src/tools.rs", *sorted((ROOT / "prompts").glob("*.md"))]
    inventory = {p.relative_to(ROOT).as_posix(): sha(p) for p in build_sources}
    inventory_path = out / ("resume-build-inventory.json" if args.resume else "build-inventory.json")
    save(inventory_path, inventory)

    def command(name, argv, expected=(0,), env=None, timeout=120):
        process = subprocess.run([str(x) for x in argv], cwd=ROOT, env=env,
                                 capture_output=True, timeout=timeout)
        (out / (name + ".stdout.json")).write_bytes(process.stdout)
        (out / (name + ".stderr.log")).write_bytes(process.stderr)
        records.append(dict(command=[str(x) for x in argv], exitCode=process.returncode,
                            stdout=name + ".stdout.json", stderr=name + ".stderr.log"))
        save(out / "commands.json", records)
        assert process.returncode in expected, (name, process.returncode)
        return json.loads(process.stdout)

    build = subprocess.run(["cargo", "build", "-p", "project-knowledge-cli", "--locked"],
                           cwd=ROOT, capture_output=True, timeout=600)
    (out / "build.log").write_bytes(build.stdout + build.stderr)
    assert build.returncode == 0
    binary = ROOT / "target/debug" / ("project-knowledge.exe" if os.name == "nt" else "project-knowledge")
    frozen = ROOT / "target/verified-cli" / sha(binary) / binary.name
    frozen.parent.mkdir(parents=True, exist_ok=True)
    if not frozen.exists():
        shutil.copy2(binary, frozen)
    assert sha(frozen) == sha(binary)
    truth = json.loads((ROOT / "fixtures/truth/scenario2.json").read_text(encoding="utf-8"))
    manifest = json.loads((ROOT / "fixtures/truth/manifest.json").read_text(encoding="utf-8"))
    original = ROOT / truth["auditedModel"]["source"]
    assert sha(original) == manifest["files"][truth["auditedModel"]["source"]]
    # This oracle is saved before model generation and is never available to its tools.
    oracle = dict(truthHash=manifest["truthHash"], sourceHash=sha(original),
                  states=truth["auditedModel"]["states"], keys=truth["auditedModel"]["compositeKeys"],
                  transitions=[
                      ["DRAFT", "SUBMITTED", "submit", "orderedQty > 0 AND supplierId present."],
                      ["SUBMITTED", "APPROVED", "approve", "role == buyer AND approvalGranted."],
                      ["SUBMITTED", "DRAFT", "reject or withdraw", "applicant == actor for withdraw."],
                      ["APPROVED", "RECEIVING", "receive", "sum(POSTED, not-deleted receipt qty) > 0."],
                      ["RECEIVING", "COMPLETED", "finish", "netReceived >= orderedQty AND all receipts valid."],
                      ["APPROVED", "CANCELLED", "cancel", "netReceived == 0."],
                  ], documents=[["purchase", "receipt"], ["receipt", "return"]])
    oracle_path = out / "independent-oracle.json"
    if oracle_path.exists():
        assert json.loads(oracle_path.read_text(encoding="utf-8")) == oracle
    else:
        save(oracle_path, oracle)
    if not args.resume:
        assert not config.exists(), "Use a new attempt name to preserve previous runs"
        config.parent.mkdir(parents=True)
        shutil.copy2(original, project / "rules.md")
        save(config, dict(schemaVersion=1, projectId="T19-audited-purchase", repositories=[dict(id="business", root="..")], selection={}))
        command("configure", [frozen, "agent", "configure", "--config", config, "--env-file", ROOT / ".env",
                              "--max-turns", "48", "--max-tool-calls", "100", "--timeout-seconds", "900",
                              "--max-input-chars", "500000", "--max-output-tokens", "12000"])
        command("index-base", [frozen, "index", "--config", config, "--base"], (0, 2))
    goal = (
        "Investigate the current rules.md and create an independent source-driven Node workflow collector, then a complete source-cited Skill in this SAME run. "
        "Scope is the explicitly labelled audited acceptance model, never the original synes business. Core/SDK must not change. "
        "Call code-map {}, export-snapshot include [rules.md], read-source, import-conventions notes/sourceIds. "
        "Inspect output/case/manifest and identity schemas as needed. Use includeIdentityHelper=true, list identity.mjs in artifacts; "
        "the helper exports computeEntityId(projectId,kind,orderedSlots), computeRelationId(projectId,kind,from,to,qualifiers), computeEvidenceId(sourceId,hash,start,end,role). "
        "orderedSlots are {name,value} objects, never a dictionary. workflowState slots scopeId,workflowId,semanticKey; "
        "documentType slots scopeId,semanticKey. Scope comes from configSubset, project ID also; choose one stable workflowId for this document. "
        "Implement a SMALL text grammar for documented lines STATE -> STATE: trigger, guard. Recompute states, edges and exact trigger/guard text from EACH current source; "
        "also parse the documented Documents chain and its ordered key names, emitting documentType entities and relates_document edges with qualifiers.orderedKeys as those names. "
        "Preserve the source words rather than interpreting code. Use declared/static assertions with actual UTF-8 evidence. Do not turn the conflict variant sentence, logging order or /help into transitions. "
        "This collector need not parse TS/SQL or invent router or column bindings. Handle empty input/deleted source as successful empty replacement; never hard-code states/paths/conditions as output. "
        "Coverage capability workflow, sourceSetId from the input sourceSets, complete for the supported current declarations; sources missing should yield empty output. "
        "One extract stage with rules.md selector and at least one meaningful case requiring real computed entity IDs and a real relation ID. "
        "Case input is the exact exported CollectorInput with pluginId/version/stageId adjusted to the manifest. "
        "Write manifest, collector, case and input in a bundle, validate-plugin, repair only structured errors within budget. "
        "After activation CONTINUE export-topic-context topicId=purchase-workflow query=purchase view=workflow maxChars=80000. "
        "Import a concise TopicArtifact preserving the returned metadata and ALL gaps. Each explanatory line has [[claim:id]], original evidenceIds from context.sourceReferences. "
        "Use basis=declared,origin=static for original document claims; no AI-inferred claims without required gaps. "
        "Then export-skill name=purchase-workflow. Do not stop at activation; finish only with a fresh complete Skill."
        " IMPORTANT implementation constraints: keep the collector below 120 lines. Do not implement glob matching, framework logic, generic config validation or extra capabilities. "
        "Sources are ALREADY selected by the host. Output dependencies:[]; the Core records and validates host source/sourceSet dependencies itself. "
        "Do not guess source-set dependency hashes/keys from selector IDs. If you do emit dependencies, they must exactly match the exported key and digest, so prefer the empty list. "
        "Use a regex with exactly THREE capture groups for transition lines: from, to, and rest; the rest is match[3], not match[4]. "
        "Split rest at its FIRST comma into trigger and guard. Do not reword conditions. Parse the Documents chain and ordered key tokens directly from that current line. "
        "No Core/SDK template collectors or output facts may be copied; generate the small document parser yourself from this project's current source."
    )
    argv = [frozen, "agent", "resume" if args.resume else "build", "--config", config, "--env-file", ROOT / ".env"]
    argv += ["--run", args.resume] if args.resume else ["--focus", "workflow", "--with-skill", "--goal", goal]
    result = command("resume" if args.resume else "real-build", argv, (0, 1, 2), timeout=1000)
    save(out / "live-summary.json", dict(status=result.get("status"), runId=result.get("runId"),
         modelTurns=result.get("modelTurns"), toolCalls=result.get("toolCalls"), cli=str(frozen), cliSha256=sha(frozen),
         oracleSha256=sha(oracle_path), sourceHash=sha(project / "rules.md"),
         sourceHashes=json.loads(inventory_path.read_text(encoding="utf-8")) if inventory_path.exists() else inventory))
    print(json.dumps(dict(status=result.get("status"), runId=result.get("runId"), modelTurns=result.get("modelTurns"),
                          toolCalls=result.get("toolCalls"), issues=[i.get("error", {}).get("code") for i in result.get("issues", [])])), flush=True)
    assert result.get("status") == "complete", "Inspect retained structured issues; do not alter independent truth"
    assert result["skill"]["freshness"] == "fresh" and result["skill"]["status"] == "complete"
    database = config.parent / "index.sqlite"

    def facts():
        with sqlite3.connect(database.resolve().as_uri() + "?mode=ro", uri=True) as db:
            entities = db.execute("""SELECT e.id,e.kind,e.identity_json,a.name,a.attributes_json,b.state
                FROM entities e JOIN entity_assertions a ON a.entity_id=e.id
                JOIN current_batches c ON c.batch_id=a.batch_id JOIN batches b ON b.id=a.batch_id
                WHERE e.kind IN ('workflowState','documentType') ORDER BY e.id""").fetchall()
            relations = db.execute("""SELECT r.kind,r.from_id,r.to_id,r.qualifiers_json,b.state
                FROM relations r JOIN relation_assertions a ON a.relation_id=r.id
                JOIN current_batches c ON c.batch_id=a.batch_id JOIN batches b ON b.id=a.batch_id
                WHERE r.kind IN ('transitions_to','relates_document') ORDER BY r.id""").fetchall()
            refs = db.execute("""SELECT e.id,e.content_hash,e.start_byte,e.end_byte,s.path
                FROM evidence e JOIN assertion_evidence ae ON ae.evidence_id=e.id
                JOIN assertions a ON a.id=ae.assertion_id JOIN current_batches c ON c.batch_id=a.batch_id
                JOIN sources s ON s.id=e.source_id WHERE a.producer_key NOT LIKE 'core.%' ORDER BY e.id""").fetchall()
        names = {e[0]: json.loads(e[2])[-1] for e in entities}
        return dict(states=sorted(names[e[0]] for e in entities if e[1] == "workflowState"),
                    entities=entities,
                    transitions=sorted([[names[r[1]], names[r[2]], json.loads(r[3]).get("trigger"), json.loads(r[3]).get("guard")] for r in relations if r[0] == "transitions_to"]),
                    documents=sorted([[names[r[1]], names[r[2]], json.loads(r[3])] for r in relations if r[0] == "relates_document"]),
                    relations=relations, evidence=refs)

    actual = facts()
    save(out / "actual-business.json", actual)
    assert actual["states"] == sorted(oracle["states"]), actual["states"]
    normalize = lambda s: s.strip().rstrip(".").strip()
    assert sorted([[*r[:3], normalize(r[3])] for r in actual["transitions"]]) == sorted([[*r[:3], normalize(r[3])] for r in oracle["transitions"]]), actual["transitions"]
    assert sorted([r[:2] for r in actual["documents"]]) == sorted(oracle["documents"])
    assert all(r[2].get("orderedKeys") == oracle["keys"] for r in actual["documents"]), actual["documents"]
    assert all(e[-1] == "fresh" for e in actual["entities"])
    assert all(r[-1] == "fresh" for r in actual["relations"])
    assert actual["evidence"] and all(e[1] == sha(project / e[4]) and 0 <= e[2] < e[3] <= (project / e[4]).stat().st_size for e in actual["evidence"])
    command("workflow-query", [frozen, "query", "--config", config, "--operation", "workflow", "--text", "purchase"], (0, 2))
    skill = Path(result["skill"]["directory"])
    skill_manifest = json.loads((skill / "references/index-manifest.json").read_text(encoding="utf-8"))
    assert all(sha(skill / path) == value for path, value in skill_manifest["files"].items())
    shutil.copytree(skill, out / "verified-skill", dirs_exist_ok=False)
    # No model entry or .env loading: disable the component and remove all provider env vars.
    no_model = {k: v for k, v in os.environ.items() if not k.startswith("PROJECT_KNOWLEDGE_")}
    saved_config = config.read_bytes()
    value = json.loads(saved_config)
    value["agent"]["enabled"] = False
    save(config, value)
    source = project / "rules.md"
    saved_source = source.read_bytes()
    model_turns = result["modelTurns"]
    observations = []
    try:
        source.write_text(saved_source.decode("utf-8").replace("netReceived == 0", "netReceived <= 0"), encoding="utf-8")
        incremental = command("no-model-change", [frozen, "index", "--config", config, "--incremental"], (0, 2), env=no_model)
        changed = facts()
        assert any(r[0:3] == ["APPROVED", "CANCELLED", "cancel"] and "<= 0" in r[3] for r in changed["transitions"])
        observations.append(dict(case="guard-change", response=incremental, actual=changed))
        source.unlink()
        incremental = command("no-model-delete", [frozen, "index", "--config", config, "--incremental"], (0, 2), env=no_model)
        assert facts()["states"] == [] and facts()["transitions"] == []
        observations.append(dict(case="source-deletion", response=incremental, actual=facts()))
    finally:
        config.write_bytes(saved_config)
        source.write_bytes(saved_source)
        command("no-model-restore", [frozen, "index", "--config", config, "--incremental"], (0, 2), env=no_model)
    assert facts()["states"] == sorted(oracle["states"])
    saved = json.loads((config.parent / "work/agent" / result["runId"] / "run.json").read_text(encoding="utf-8"))
    assert saved["modelTurns"] == model_turns
    status = command("after-incremental-status", [frozen, "query", "--config", config, "--operation", "status"], (0, 2), env=no_model)
    assert all(e["freshness"] == "stale" for e in status["data"]["knowledge"]["exports"])
    save(out / "acceptance-report.json", dict(passed=True, oracle=oracle, actual=actual,
         noModelIncremental=observations, noModelRequests=0, modelTurnsBefore=model_turns, modelTurnsAfter=saved["modelTurns"],
         skillDirectory=str(skill), skillManifest=skill_manifest, scope="T06 audited workflow document sample; production and other scenarios not claimed"))
    print("T19 real business, knowledge and no-model incremental acceptance passed", flush=True)


if __name__ == "__main__":
    main()
