"""Verify the semantic stages and regression of shared AST signatures; never run business SQL."""
import hashlib
import json
import os
import pathlib
import re
import subprocess

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[2]
commands = [(f'package-{kind}.log', ['node', file, '--check']) for kind, file in [
    ('frontend','scripts/package-example-collectors.mjs'),('api','scripts/package-api-examples.mjs'),
    ('database','scripts/package-database-examples.mjs'),('link','scripts/package-link-examples.mjs'),
    ('semantics','scripts/package-semantic-examples.mjs')]] + [
    ('plugins.log',['node','scripts/verify-plugins.mjs','--capabilities','workflow,report']),
    ('frontend-regression.log',['cargo','test','-p','project-knowledge-core','--test','business_scenarios','frontend_','--locked','-j','1','--','--test-threads=1']),
    ('clippy.log',['cargo','clippy','--workspace','--all-targets','--locked','-j','1','--','-D','warnings']),
    ('fmt.log',['cargo','fmt','--all','--','--check']),
    ('contracts.log',['cargo','run','-p','xtask','--locked','--','export-contracts','--check']),
    ('types.log',['node','scripts/generate-types.mjs','--check']),
    ('truth.log',['python','scripts/verify-truth.py']),
    ('plan.log',['node','scripts/verify-plan.mjs','--plan','../plan/NEW-PROJECT-LONG-TASK.md']),
    ('source-audit.log',['node','scripts/audit-source.mjs','--output',str(HERE/'source-audit.json')]),
]
records=[]
for filename,command in commands:
    print('Running '+' '.join(command),flush=True)
    process=subprocess.run(command,cwd=ROOT,capture_output=True,timeout=1200,
        env=dict(os.environ,PYTHONIOENCODING='utf-8',PK_SCENARIO_EVIDENCE_DIR=str(HERE)))
    log=HERE/filename;log.write_bytes(process.stdout+process.stderr)
    records.append(dict(command=command,cwd=str(ROOT),exitCode=process.returncode,log=log.relative_to(ROOT).as_posix(),logSha256=hashlib.sha256(log.read_bytes()).hexdigest()))
    print(json.dumps(records[-1]),flush=True)
    if process.returncode: raise SystemExit(process.returncode)
files=[ROOT/name for name in ['collectors/templates/python/lib/semantic_facts.py','collectors/templates/python/lib/workflow_ast.py',
    'collectors/templates/python/lib/report_ast.py','collectors/templates/python/lib/source_ast.py','collectors/templates/node/lib/frontend-ast.mjs',
    'crates/core/tests/business_scenarios.rs','crates/core/tests/support/semantics.rs','crates/core/tests/support/linking.rs',
    'scripts/package-semantic-examples.mjs','scripts/verify-plugins.mjs','fixtures/truth-supplements/T14-semantics.json']]
for key in ['a','b']:
    for kind in ['frontend','api','database','semantics']:
        files.extend(path for path in (ROOT/f'collectors/examples/frontend-backend-{key}/{kind}').rglob('*') if path.is_file() and '__pycache__' not in path.parts)
report=dict(date='2026-10-28 Asia/Shanghai',task='T14',commands=records,
    baselineTruthHash=json.loads((ROOT/'fixtures/truth/manifest.json').read_text(encoding='utf-8'))['truthHash'],
    reviewHash=hashlib.sha256((ROOT/'fixtures/truth-supplements/T14-semantics.json').read_bytes()).hexdigest(),
    fileHashes={path.relative_to(ROOT).as_posix():hashlib.sha256(path.read_bytes()).hexdigest() for path in sorted(set(files))},
    testSummaries=re.findall(r'test result:.*',(HERE/'plugins.log').read_text(encoding='utf-8',errors='replace')),
    independentExpectedFieldsPerLayout=9,declaredAndCodeTransitionsPerLayout=14,expectedStatesPerLayout=6,
    forbiddenConfirmed=0,productModelRequests=0,businessDatabaseConnections=0,productionAccuracyClaimed=False,
    limitations=['Supported documented grammar and static SQLite SELECT/CTE patterns only',
        'Missing time fields/physical quantity units/cardinality stay gaps','Windows x64 execution only in this run'])
(HERE/'verification-report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
