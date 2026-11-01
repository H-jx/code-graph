"""Reproduce T13 with the locked workspace, original truth and bounded logs."""
import hashlib
import json
import os
import pathlib
import re
import subprocess

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[2]
commands = [
    ('artifacts.log', ['node', 'scripts/package-link-examples.mjs', '--check']),
    ('workspace-tests.log', ['cargo', 'test', '--workspace', '--locked', '-j', '1', '--', '--test-threads=1']),
    ('clippy.log', ['cargo', 'clippy', '--workspace', '--all-targets', '--locked', '-j', '1', '--', '-D', 'warnings']),
    ('fmt.log', ['cargo', 'fmt', '--all', '--', '--check']),
    ('contracts.log', ['cargo', 'run', '-p', 'xtask', '--locked', '--', 'export-contracts', '--check']),
    ('types.log', ['node', 'scripts/generate-types.mjs', '--check']),
    ('type-boundaries.log', ['node', 'scripts/verify-contract-types.mjs']),
    ('typecheck.log', ['powershell', '-NoProfile', '-Command', 'pnpm run typecheck'] if os.name == 'nt' else ['pnpm', 'run', 'typecheck']),
    ('truth.log', ['python', 'scripts/verify-truth.py']),
    ('plan.log', ['node', 'scripts/verify-plan.mjs', '--plan', '../plan/NEW-PROJECT-LONG-TASK.md']),
    ('source-audit.log', ['node', 'scripts/audit-source.mjs', '--output', str(HERE / 'source-audit.json')]),
]
results = []
for name, command in commands:
    print('Running ' + ' '.join(command), flush=True)
    process = subprocess.run(command, cwd=ROOT, capture_output=True, timeout=1200,
        env=dict(os.environ, PYTHONIOENCODING='utf-8', PK_SCENARIO_EVIDENCE_DIR=str(HERE)))
    log = HERE / name
    log.write_bytes(process.stdout + process.stderr)
    results.append(dict(command=command, cwd=str(ROOT), exitCode=process.returncode,
        log=log.relative_to(ROOT).as_posix(), logSha256=hashlib.sha256(log.read_bytes()).hexdigest()))
    print(json.dumps(results[-1]), flush=True)
    if process.returncode:
        raise SystemExit(process.returncode)
files = [ROOT / name for name in ['crates/core/src/contract/collector.rs', 'crates/core/src/collect/import.rs', 'crates/core/src/collect/validate.rs',
    'crates/core/tests/collectors.rs', 'crates/core/tests/business_scenarios.rs', 'crates/core/tests/support/linking.rs',
    'collectors/templates/node/lib/link-facts.mjs', 'collectors/templates/node/lib/frontend-ast.mjs',
    'scripts/package-link-examples.mjs', 'scripts/verify-plugins.mjs', 'docs/protocol.md',
    'docs/schema/contract.schema.json', 'docs/schema/protocol.json', 'packages/sdk/src/generated/contract.ts',
    'fixtures/truth-supplements/T13-link.json']]
for key in ['a','b']:
    for capability in ['link','frontend','api']:
        files.extend(path for path in (ROOT/f'collectors/examples/frontend-backend-{key}/{capability}').rglob('*')
            if path.is_file() and '__pycache__' not in path.parts)
report = dict(date='2026-10-28 Asia/Shanghai', task='T13', commands=results,
    baselineTruthHash=json.loads((ROOT/'fixtures/truth/manifest.json').read_text(encoding='utf-8'))['truthHash'],
    reviewHash=hashlib.sha256((ROOT/'fixtures/truth-supplements/T13-link.json').read_bytes()).hexdigest(),
    fileHashes={path.relative_to(ROOT).as_posix():hashlib.sha256(path.read_bytes()).hexdigest() for path in sorted(set(files))},
    testSummaries=re.findall(r'test result:.*', (HERE/'workspace-tests.log').read_text(encoding='utf-8',errors='replace')),
    expectedCrossRepositoryBindings=4, forbiddenConfirmed=0, productModelRequests=0,
    businessDatabaseConnections=0, productionAccuracyClaimed=False,
    limitations=['Supported example registrations and binding shapes only', 'Windows x64 execution in this run; four platform install remains T23'])
(HERE/'verification-report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
