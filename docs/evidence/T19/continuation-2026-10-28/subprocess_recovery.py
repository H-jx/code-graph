"""Kill an owned CLI after activation, then resume knowledge through a local provider."""
import argparse
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import threading

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--business", default="business-01")
    parser.add_argument("--cli")
    parser.add_argument("--attempt", default="subprocess-recovery-03")
    args = parser.parse_args()
    business = HERE / args.business
    completed = business / "citation-repair.stdout.json"
    if not completed.exists():
        completed = business / "repair.stdout.json"
    live_path = completed if completed.exists() and completed.stat().st_size and "activation" in json.loads(completed.read_text(encoding="utf-8")) else next(p for p in (business / "project/.project-knowledge/work/agent").glob("agent_*/run.json") if "activation" in json.loads(p.read_text(encoding="utf-8")))
    live = json.loads(live_path.read_text(encoding="utf-8"))
    assert live["activation"]["status"] == "active"
    cli = args.cli or json.loads((business / "commands.json").read_text(encoding="utf-8"))[0]["command"][0]
    revision = business / "project/.project-knowledge/work/agent" / live["runId"] / f"revision-{live['revision']:03}"
    files = [dict(path=p.relative_to(revision).as_posix(), content=p.read_text(encoding="utf-8")) for p in revision.rglob("*") if p.is_file()]
    out = HERE / args.attempt
    out.mkdir()
    project = out / "project"
    config = project / ".project-knowledge/project.json"
    config.parent.mkdir(parents=True)
    (project / "rules.md").write_bytes((business / "project/rules.md").read_bytes())
    if (business / "project/flow.ts").exists():
        (project / "flow.ts").write_bytes((business / "project/flow.ts").read_bytes())
    secret = "T19-local-provider-test-key"
    credential = out / "test.env"
    credential.write_text(f"T19_LOCAL_KEY={secret}\n", encoding="utf-8")
    interrupted = threading.Event()
    release = threading.Event()
    trace = []
    data = dict(refs=[], input=None)

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def do_POST(self):
            body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            step = len(trace)
            last = next((json.loads(m["content"]) for m in reversed(body["messages"]) if m["role"] == "tool"), {})
            trace.append(dict(request=step + 1, lastToolError=last.get("error", {}).get("code")))
            tool = None
            if step == 0:
                tool = ("code-map", {})
            elif step == 1:
                tool = ("export-snapshot", dict(include=["rules.md"]))
            elif step == 2:
                data.update(refs=last["sourceReferences"], input=last["input"])
                tool = ("read-source", dict(sourceId=data["refs"][0]["sourceId"]))
            elif step == 3:
                tool = ("import-conventions", dict(notes="Original audited workflow document; local provider tests recovery only.", sourceIds=[data["refs"][0]["sourceId"]]))
            elif step == 4:
                tool = ("write-candidate", dict(files=files))
            elif step == 5:
                tool = ("validate-plugin", {})
            elif step == 6:
                assert last["activation"]["status"] == "active", last
                interrupted.set()
                assert release.wait(30)
            elif step == 7:
                tool = ("export-topic-context", dict(topicId="recovered-purchase", query="flow.ts", view="search", kinds=["operation", "symbol"], maxChars=120000))
            elif step == 8:
                assert last.get("inputDigest"), last
                ref = max(last["sourceReferences"], key=lambda r: r["endByte"] - r["startByte"])
                text = "The audited purchase document declares a workflow."
                artifact = {k: last[k] for k in ["protocolVersion", "snapshot", "inputDigest", "promptHash", "templateHash", "sourceReferences", "gaps"]}
                artifact.update(topicId="recovered-purchase", title="Recovered audited purchase", markdown=text + " [[claim:source]]",
                                claims=[dict(id="source", text=text, basis="declared", origin="static", evidenceRefs=[ref["evidenceId"]], gapIds=[])])
                tool = ("import-topic", artifact)
            elif step == 9:
                assert last.get("version"), last
                tool = ("export-skill", dict(name="recovered-purchase"))
            elif step == 10:
                assert last.get("freshness") == "fresh" and last.get("status") == "complete", last
            else:
                raise AssertionError("unexpected model call")
            message = dict(role="assistant", content="OK")
            if tool:
                message = dict(role="assistant", content=None, tool_calls=[dict(id=f"call-{step}", type="function", function=dict(name=tool[0], arguments=json.dumps(tool[1])))])
            value = dict(id=f"local-{step}", object="chat.completion", created=1, model="local-recovery", choices=[dict(index=0, message=message, finish_reason="tool_calls" if tool else "stop")], usage=dict(prompt_tokens=1, completion_tokens=1, total_tokens=2))
            payload = json.dumps(value).encode("utf-8")
            try:
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(payload)))
                self.end_headers()
                self.wfile.write(payload)
            except (BrokenPipeError, ConnectionResetError, ConnectionAbortedError):
                pass

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    original = json.loads((business / "project/.project-knowledge/project.json").read_text(encoding="utf-8"))
    original["agent"] = dict(enabled=True, provider="openai-compatible", model="local-recovery", baseUrl=f"http://127.0.0.1:{server.server_port}/v1",
                             apiKeyEnv="T19_LOCAL_KEY", maxTurns=24, maxToolCalls=80, timeoutSeconds=180, maxInputChars=500000, maxOutputTokens=1000)
    config.write_text(json.dumps(original, ensure_ascii=False, indent=2), encoding="utf-8")
    environment = {k: v for k, v in os.environ.items() if not k.startswith("PROJECT_KNOWLEDGE_")}
    index = subprocess.run([cli, "index", "--config", str(config), "--base"], cwd=ROOT, env=environment, capture_output=True, timeout=60)
    assert index.returncode in (0, 2)
    process = subprocess.Popen([cli, "agent", "build", "--config", str(config), "--env-file", str(credential), "--with-skill", "--goal", "Recover current source-cited purchase knowledge"], cwd=ROOT, env=environment, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    try:
        assert interrupted.wait(45), "did not reach the activation checkpoint"
        path = next((config.parent / "work/agent").glob("agent_*/run.json"))
        before = json.loads(path.read_text(encoding="utf-8"))
        assert before["activation"]["status"] == "active" and before["modelTurns"] == 7
        # This PID was returned by our Popen. No process name search or broad kill.
        process.kill()
        stdout, stderr = process.communicate(timeout=15)
        (out / "killed.stdout.log").write_bytes(stdout)
        (out / "killed.stderr.log").write_bytes(stderr)
        release.set()
        resumed = subprocess.run([cli, "agent", "resume", "--config", str(config), "--env-file", str(credential), "--run", before["runId"]], cwd=ROOT, env=environment, capture_output=True, timeout=90)
        (out / "resumed.stdout.json").write_bytes(resumed.stdout)
        (out / "resumed.stderr.log").write_bytes(resumed.stderr)
        after = json.loads(resumed.stdout)
        assert resumed.returncode == 0 and after["status"] == "complete", after
        assert after["deadlineMs"] == before["deadlineMs"]
        assert after["modelTurns"] == 11 and after["toolCalls"] == before["toolCalls"] + 3
        assert after["activation"] == before["activation"]
        assert after["skill"]["freshness"] == "fresh"
        assert secret not in resumed.stdout.decode("utf-8") and secret not in path.read_text(encoding="utf-8")
        with sqlite3.connect(config.parent.joinpath("index.sqlite").resolve().as_uri() + "?mode=ro", uri=True) as db:
            assert db.execute("PRAGMA integrity_check").fetchone()[0] == "ok"
            assert db.execute("SELECT count(*) FROM activations WHERE status='committed'").fetchone()[0] == 1
        report = dict(passed=True, ownedPid=process.pid, killedExitCode=process.returncode, checkpoint="after committed activation, before first knowledge tool",
                      before=before, after=after, trace=trace, realExternalModelRequests=0,
                      cli=cli, cliSha256=hashlib.sha256(Path(cli).read_bytes()).hexdigest())
        (out / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print("Owned CLI kill and knowledge resume passed; one activation, same durable deadline", flush=True)
    finally:
        release.set()
        if process.poll() is None:
            process.kill()
            process.wait(timeout=15)
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)


if __name__ == "__main__":
    main()
