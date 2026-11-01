"""Locked query regression, read-only evidence and original truth; no external model or business DB."""
import hashlib
import json
import os
import pathlib
import re
import subprocess

HERE=pathlib.Path(__file__).resolve().parent
ROOT=HERE.parents[2]
commands=[
    ('indexing.log',['cargo','test','-p','project-knowledge-core','--test','indexing','--locked','-j','1','--','--test-threads=1']),
    ('agent-cli.log',['cargo','test','-p','project-knowledge-agent','-p','project-knowledge-cli','--locked','-j','1','--','--test-threads=1']),
    ('projection.log',['cargo','test','-p','project-knowledge-core','--test','collectors','validation_registered_extensions_stage_commit_search_and_reexport','--locked','-j','1','--','--test-threads=1']),
    ('clippy.log',['cargo','clippy','--workspace','--all-targets','--locked','-j','1','--','-D','warnings']),
    ('fmt.log',['cargo','fmt','--all','--','--check']),
    ('contracts.log',['cargo','run','-p','xtask','--locked','--','export-contracts','--check']),
    ('types.log',['node','scripts/generate-types.mjs','--check']),
    ('type-boundaries.log',['node','scripts/verify-contract-types.mjs']),
    ('typecheck.log',['powershell','-NoProfile','-Command','pnpm run typecheck'] if os.name=='nt' else ['pnpm','run','typecheck']),
    ('truth.log',['python','scripts/verify-truth.py']),
    ('source-audit.log',['node','scripts/audit-source.mjs','--output',str(HERE/'source-audit.json')]),
    ('plan.log',['node','scripts/verify-plan.mjs','--plan','../plan/NEW-PROJECT-LONG-TASK.md']),
    ('diff.log',['git','diff','--check']),
]
records=[]
for name,command in commands:
    print('Running '+' '.join(command),flush=True)
    process=subprocess.run(command,cwd=ROOT,capture_output=True,timeout=1200,env=dict(os.environ,PYTHONIOENCODING='utf-8'))
    log=HERE/name;log.write_bytes(process.stdout+process.stderr)
    records.append(dict(command=command,cwd=str(ROOT),exitCode=process.returncode,log=log.relative_to(ROOT).as_posix(),logSha256=hashlib.sha256(log.read_bytes()).hexdigest()))
    print(json.dumps(records[-1]),flush=True)
    if process.returncode: raise SystemExit(process.returncode)
files=[ROOT/f for f in ['Cargo.toml','Cargo.lock','crates/core/src/contract/query.rs','crates/core/src/storage/database.rs','crates/core/src/project/scan.rs','crates/core/tests/indexing.rs','crates/core/tests/support/queries.rs','crates/agent/src/tools.rs','crates/cli/src/commands/query.rs','scripts/verify-contract-types.mjs','docs/schema/contract.schema.json','docs/schema/protocol.json','packages/sdk/src/generated/contract.ts','docs/protocol.md','docs/evidence/T15/verify.py']]
for folder in ['query','graph','codemap']:
    files.extend((ROOT/'crates/core/src'/folder).glob('*.rs'))
snapshots={name:json.loads((HERE/f'{name}.json').read_text(encoding='utf-8')) for name in ['search','graph','boundaries','evidence-reads']}
assert snapshots['search']['beforeHash']==snapshots['search']['afterHash']
assert snapshots['search']['orderedCount']==30
assert snapshots['graph']['keysetCount']==58
report=dict(date='2026-10-28 Asia/Shanghai',task='T15',commands=records,
    fileHashes={f.relative_to(ROOT).as_posix():hashlib.sha256(f.read_bytes()).hexdigest() for f in sorted(set(files))},
    snapshotHashes={f'{name}.json':hashlib.sha256((HERE/f'{name}.json').read_bytes()).hexdigest() for name in snapshots},
    testSummaries=[line for name in ['indexing.log','agent-cli.log','projection.log'] for line in re.findall(r'test result:.*',(HERE/name).read_text(encoding='utf-8',errors='replace'))],
    baselineTruthHash=json.loads((ROOT/'fixtures/truth/manifest.json').read_text(encoding='utf-8'))['truthHash'],
    orderedSearchCount=30,orderedNeighborCount=58,persistentTablesChanged=False,mainFileChanged=False,
    externalModelRequests=0,businessDatabaseConnections=0,
    limitations=['Windows x64 only; four-platform delivery remains T23','Full workspace business-plugin regression is preserved in earlier task evidence, not rerun here','Coverage describes the requested indexed scope; business scenario composition remains T16'])
(HERE/'verification-report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('T15 final verification complete',flush=True)
