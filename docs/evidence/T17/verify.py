"""Reproducible T17 acceptance. Preserve failures; publish a report only on success."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
env=dict(os.environ,PYTHONIOENCODING='utf-8',PK_KNOWLEDGE_EVIDENCE_DIR=str(HERE))
reuse='--reuse-workspace' in sys.argv
previous=json.loads((HERE/'verification-report.json').read_text(encoding='utf-8')) if reuse else None
if reuse:
    baseline=json.loads((HERE/'source-audit-preliminary.json').read_text(encoding='utf-8'))['fileHashes']
    changed=[name for name,expected in baseline.items() if sha(ROOT/name)!=expected]
    assert set(changed)<=set(['crates/core/src/knowledge/context.rs','crates/core/tests/support/knowledge.rs']),changed
    inherited=next(c for c in previous['commands'] if c['log'].endswith('/workspace-tests.log'))
    assert inherited['exitCode']==0 and sha(ROOT/inherited['log'])==inherited['logSha256']
commands=[
    ('cli-acceptance.log',[sys.executable,str(HERE/'cli_verify.py')]),
    ('workspace-tests.log',['cargo','test','--workspace','--locked','-j','1','--','--test-threads=1']),
    ('indexing-final.log',['cargo','test','-p','project-knowledge-core','--test','indexing','--locked','-j','1','--','--test-threads=1']),
    ('clippy.log',['cargo','clippy','--workspace','--all-targets','--locked','-j','1','--','-D','warnings']),
    ('fmt.log',['cargo','fmt','--all','--','--check']),
    ('contracts.log',['cargo','run','-p','xtask','--locked','--','export-contracts','--check']),
    ('types.log',['node','scripts/generate-types.mjs','--check']),
    ('type-boundaries.log',['node','scripts/verify-contract-types.mjs']),
    ('typecheck.log',['powershell','-NoProfile','-Command','pnpm run typecheck'] if os.name=='nt' else ['pnpm','run','typecheck']),
    ('truth.log',[sys.executable,'scripts/verify-truth.py']),
    ('source-audit.log',['node','scripts/audit-source.mjs','--output',str(HERE/'source-audit.json')]),
    ('plan.log',['node','scripts/verify-plan.mjs','--plan','../plan/NEW-PROJECT-LONG-TASK.md']),
    ('diff.log',['git','diff','--check']),
]
records=[]
for name,command in commands:
    if reuse and name=='workspace-tests.log':
        records.append(dict(inherited,reused=True,baselineInventory='docs/evidence/T17/source-audit-preliminary.json',finalChangedSources=changed,finalCoverage='indexing-final.log and cli-acceptance.log; all other product source hashes unchanged'))
        continue
    print('Running '+' '.join(command),flush=True)
    process=subprocess.run(command,cwd=ROOT,capture_output=True,timeout=1800,env=env)
    log=HERE/name;log.write_bytes(process.stdout+process.stderr)
    record=dict(command=command,cwd=str(ROOT),exitCode=process.returncode,log=log.relative_to(ROOT).as_posix(),logSha256=sha(log));records.append(record)
    print(json.dumps(record),flush=True)
    if process.returncode:raise SystemExit(process.returncode)
summaries=re.findall(r'test result:.*',(HERE/'workspace-tests.log').read_text(encoding='utf-8',errors='replace'))
matched=sum(int(x) for x in re.findall(r'test result: ok\. (\d+) passed', '\n'.join(summaries)))
assert matched>0 and 'knowledge::knowledge_' in (HERE/'workspace-tests.log').read_text(encoding='utf-8')
cli=json.loads((HERE/'cli-report.json').read_text(encoding='utf-8'))
assert sha(Path(cli['cliArtifactPath']))==cli['cliSha256']
files=[ROOT/'Cargo.lock',ROOT/'crates/core/migrations/0003_knowledge.sql',ROOT/'crates/core/src/contract/knowledge.rs',ROOT/'crates/core/src/query/mod.rs',ROOT/'crates/core/src/storage/database.rs',ROOT/'crates/core/src/storage/knowledge.rs',ROOT/'crates/core/src/storage/runs.rs',ROOT/'crates/core/src/index/incremental.rs',ROOT/'crates/core/src/index/build.rs',ROOT/'crates/core/src/collect/commit.rs',ROOT/'crates/core/src/collect/registry.rs',ROOT/'crates/core/tests/indexing.rs',ROOT/'crates/core/tests/support/knowledge.rs',ROOT/'crates/cli/Cargo.toml',ROOT/'crates/cli/src/commands/generate.rs',ROOT/'xtask/src/main.rs',ROOT/'docs/schema/contract.schema.json',ROOT/'packages/sdk/src/generated/contract.ts',ROOT/'prompts/knowledge.md',HERE/'verify.py',HERE/'cli_verify.py']
for folder in ['crates/core/src/knowledge','crates/core/templates','skills/project-query','skills/project-onboarding']:
    files.extend(p for p in (ROOT/folder).rglob('*') if p.is_file())
snapshots=[p for p in HERE.rglob('*') if p.is_file() and p.suffix in ['.json','.md'] and p.name not in ['verification-report.json','README.md']]
report=dict(date='2026-10-28 Asia/Shanghai',task='T17',commands=records,testSummaries=summaries,testsPassed=matched,knowledgeTests=6,cli=cli,sourceHashes={p.relative_to(ROOT).as_posix():sha(p) for p in sorted(set(files))},snapshotHashes={p.relative_to(HERE).as_posix():sha(p) for p in snapshots},modelsCalled=0,businessDatabaseConnections=0,platform='Windows x64',productionAccuracyClaimed=False,limitations=['Dependency selection conservatively binds the selected repositories, not individual semantic statements','Only mapped Mermaid and cited explanation lines are accepted in external topics','CURRENT.json selects an immutable complete Skill directory; no in-place overwrite of a loaded Skill','Agent knowledge workflow and SDK/MCP remain subsequent tasks'])
report['finalIndexingTestSummaries']=re.findall(r'test result:.*',(HERE/'indexing-final.log').read_text(encoding='utf-8',errors='replace'))
report['workspaceReused']=reuse
(HERE/'verification-report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('T17 final acceptance passed',flush=True)
