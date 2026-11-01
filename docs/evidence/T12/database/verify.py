"""Reproduce the T12 database/Core acceptance and save commands, logs and hashes."""
import hashlib
import json
import os
import pathlib
import re
import subprocess

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[3]
commands = [
    ('plugins.log', ['node', 'scripts/verify-plugins.mjs', '--capabilities', 'api,database']),
    ('workspace-tests.log', ['cargo', 'test', '--workspace', '--locked', '-j', '1', '--', '--test-threads=1']),
    ('clippy.log', ['cargo', 'clippy', '--workspace', '--all-targets', '--locked', '-j', '1', '--', '-D', 'warnings']),
    ('fmt.log', ['cargo', 'fmt', '--all', '--', '--check']),
    ('contracts.log', ['cargo', 'run', '-p', 'xtask', '--locked', '--', 'export-contracts', '--check']),
    ('types.log', ['node', 'scripts/generate-types.mjs', '--check']),
    ('type-boundaries.log', ['node', 'scripts/verify-contract-types.mjs']),
    ('truth.log', ['python', 'scripts/verify-truth.py']),
    ('plan.log', ['node', 'scripts/verify-plan.mjs', '--plan', '../plan/NEW-PROJECT-LONG-TASK.md']),
    ('source-audit.log', ['node', 'scripts/audit-source.mjs', '--output', str(HERE / 'source-audit.json')]),
]
results = []
for name, command in commands:
    print('Running ' + ' '.join(command), flush=True)
    process = subprocess.run(command, cwd=ROOT, capture_output=True, timeout=900,
        env=dict(os.environ, PYTHONIOENCODING='utf-8', PK_SCENARIO_EVIDENCE_DIR=str(HERE)))
    log = HERE / name
    log.write_bytes(process.stdout + process.stderr)
    results.append(dict(command=command, cwd=str(ROOT), exitCode=process.returncode,
        log=log.relative_to(ROOT).as_posix(), logSha256=hashlib.sha256(log.read_bytes()).hexdigest()))
    print(json.dumps(results[-1]), flush=True)
    if process.returncode:
        raise SystemExit(process.returncode)
files = [ROOT / name for name in [
    'crates/core/tests/business_scenarios.rs', 'crates/core/tests/indexing.rs', 'crates/core/src/contract/relation.rs',
    'collectors/templates/python/lib/database_ast.py', 'collectors/templates/python/lib/source_ast.py',
    'scripts/package-database-examples.mjs', 'scripts/verify-plugins.mjs', 'scripts/audit-source.mjs',
    'docs/schema/contract.schema.json', 'docs/schema/protocol.json', 'packages/sdk/src/generated/contract.ts',
]]
for folder in ['collectors/examples/frontend-backend-a/database', 'collectors/examples/frontend-backend-b/database', 'fixtures/truth-supplements/T12-database']:
    files.extend(path for path in (ROOT / folder).rglob('*') if path.is_file() and '__pycache__' not in path.parts)
report = dict(date='2026-10-28 Asia/Shanghai', task='T12', status='done',
    baselineTruthHash=json.loads((ROOT/'fixtures/truth/manifest.json').read_text(encoding='utf-8'))['truthHash'],
    reviewHash=hashlib.sha256((ROOT/'fixtures/truth-supplements/T12-database/review.json').read_bytes()).hexdigest(),
    commands=results, fileHashes={path.relative_to(ROOT).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest() for path in sorted(set(files))},
    testSummaries=re.findall(r'test result:.*', (HERE/'workspace-tests.log').read_text(encoding='utf-8', errors='replace')),
    expectedDdlEntitiesPerLayout=90, correctOrderedJoinsPerLayout=4, forbiddenCrossDatasourceReads=0,
    productModelRequests=0, businessDatabaseConnections=0, productionAccuracyClaimed=False,
    supportedDialect='sqlite', supportedOrm='explicit lexical/imported connect/model/query/findAll; source is never executed',
    limitations=['Report/workflow business semantics remain T14', 'Runtime/dynamic SQL/ORM and undeclared bindings stay gaps',
                 'Windows x64 execution only in this run; other platforms are T23 acceptance'])
(HERE/'verification-report.json').write_text(json.dumps(report, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
