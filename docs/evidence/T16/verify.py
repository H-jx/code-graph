"""Verify the completed CLI evidence and all affected Rust/plugin/query regressions."""
import hashlib
import json
import os
import pathlib
import re
import subprocess

HERE=pathlib.Path(__file__).resolve().parent
ROOT=HERE.parents[2]
cli=json.loads((HERE/'cli-report.json').read_text(encoding='utf-8'))
for command in cli['commands']:
    assert command['exitCode']==0
    assert hashlib.sha256((ROOT/command['log']).read_bytes()).hexdigest()==command['logSha256']
binary=ROOT/cli['cliArtifactPath']
assert hashlib.sha256(binary.read_bytes()).hexdigest()==cli['cliSha256'], 'CLI evidence must match the verified binary'
commands=[
    ('business-regression.log',['cargo','test','-p','project-knowledge-core','--test','business_scenarios','--locked','-j','1','--','--test-threads=1']),
    ('indexing-final.log',['cargo','test','-p','project-knowledge-core','--test','indexing','--locked','-j','1','--','--test-threads=1']),
    ('agent-cli-regression.log',['cargo','test','-p','project-knowledge-agent','-p','project-knowledge-cli','--locked','-j','1','--','--test-threads=1']),
    ('clippy.log',['cargo','clippy','--workspace','--all-targets','--locked','-j','1','--','-D','warnings']),
    ('fmt.log',['cargo','fmt','--all','--','--check']),
    ('contracts.log',['cargo','run','-p','xtask','--locked','--','export-contracts','--check']),
    ('types.log',['node','scripts/generate-types.mjs','--check']),
    ('type-boundaries.log',['node','scripts/verify-contract-types.mjs']),
    ('typecheck.log',['powershell','-NoProfile','-Command','pnpm run typecheck'] if os.name=='nt' else ['pnpm','run','typecheck']),
    *[(f'package-{kind}.log',['node',file,'--check']) for kind,file in [('frontend','scripts/package-example-collectors.mjs'),('api','scripts/package-api-examples.mjs'),('database','scripts/package-database-examples.mjs'),('link','scripts/package-link-examples.mjs'),('semantic','scripts/package-semantic-examples.mjs')]],
    ('truth-final.log',['python','scripts/verify-truth.py']),
    ('source-audit.log',['node','scripts/audit-source.mjs','--output',str(HERE/'source-audit.json')]),
    ('plan.log',['node','scripts/verify-plan.mjs','--plan','../plan/NEW-PROJECT-LONG-TASK.md']),
    ('diff.log',['git','diff','--check']),
]
records=[]
for name,command in commands:
    print('Running '+' '.join(command),flush=True)
    # Keep final CLI comparison snapshots intact; this broad suite checks Core paths again.
    process=subprocess.run(command,cwd=ROOT,capture_output=True,timeout=1800,env=dict(os.environ,PYTHONIOENCODING='utf-8'))
    log=HERE/name;log.write_bytes(process.stdout+process.stderr)
    records.append(dict(command=command,cwd=str(ROOT),exitCode=process.returncode,log=log.relative_to(ROOT).as_posix(),logSha256=hashlib.sha256(log.read_bytes()).hexdigest()))
    print(json.dumps(records[-1]),flush=True)
    if process.returncode: raise SystemExit(process.returncode)
files=[ROOT/f for f in ['Cargo.toml','Cargo.lock','crates/core/src/search/documents.rs','crates/core/src/graph/paths.rs','crates/core/tests/business_scenarios.rs','crates/core/tests/support/scenarios.rs','crates/core/tests/support/queries.rs','crates/core/tests/support/semantics.rs','collectors/templates/python/lib/database_ast.py','scripts/verify-scenarios.mjs','docs/evidence/T16/verify.py']]
files.extend((ROOT/'crates/core/src/query').glob('*.rs'))
files.extend((ROOT/'fixtures/truth-supplements/T16-bindings').glob('*'))
for layout in ['a','b']:
    files.extend(f for f in (ROOT/f'collectors/examples/frontend-backend-{layout}/database').rglob('*') if f.is_file() and '__pycache__' not in f.parts)
snapshots=[HERE/item for item in ['a-page.json','a-workflow.json','a-report.json','b-page.json','b-workflow.json','b-report.json','a-natural-feature.json','b-natural-feature.json','a-cross-client-cursor.json','b-cross-client-cursor.json','route-ambiguity.json','workflow-code-only.json','numeric-sql.json','cli-report.json']]
report=dict(date='2026-10-28 Asia/Shanghai',task='T16',commands=records,cliCommands=cli['commands'],cliSha256=cli['cliSha256'],
    fileHashes={f.relative_to(ROOT).as_posix():hashlib.sha256(f.read_bytes()).hexdigest() for f in sorted(set(files))},
    snapshotHashes={f.name:hashlib.sha256(f.read_bytes()).hexdigest() for f in snapshots},
    testSummaries=[s for name in ['business-regression.log','indexing-final.log','agent-cli-regression.log'] for s in re.findall(r'test result:.*',(HERE/name).read_text(encoding='utf-8',errors='replace'))],
    baselineTruthHash=cli['truthHash'],cliResponses=6,layouts=2,statesPerLayout=6,transitionsPerLayout=14,reportFieldsPerLayout=9,queryColumnsPerLayout=7,
    externalModelRequests=0,businessDatabaseConnections=0,productionAccuracyClaimed=False,
    limitations=['Windows x64 only','Natural-language feature fallback is explicit lexical matching and partial, not generated business interpretation','Unknown time fields, units and cardinality stay gaps','SDK/MCP and knowledge/dependency acceptance remain their own tasks'])
(HERE/'verification-report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('T16 final verification complete',flush=True)
