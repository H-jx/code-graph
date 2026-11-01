"""Verify the T12 API subloop. This deliberately does not mark database/T12 complete."""
import hashlib
import json
import os
import pathlib
import re
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[3]
HERE = pathlib.Path(__file__).resolve().parent
commands = [
    ('plugins-api.log', ['node', 'scripts/verify-plugins.mjs', '--capabilities', 'routes,frontend,comments,modules,entrypoints,api']),
    ('workspace-tests.log', ['cargo', 'test', '--workspace', '--locked', '-j', '1', '--', '--test-threads=1']),
    ('clippy.log', ['cargo', 'clippy', '--workspace', '--all-targets', '--locked', '-j', '1', '--', '-D', 'warnings']),
    ('fmt.log', ['cargo', 'fmt', '--all', '--', '--check']),
    ('truth.log', ['python', 'scripts/verify-truth.py']),
    ('plan.log', ['node', 'scripts/verify-plan.mjs', '--plan', '../plan/NEW-PROJECT-LONG-TASK.md']),
]
results = []
for filename, command in commands:
    print('Running ' + ' '.join(command), flush=True)
    process = subprocess.run(command, cwd=ROOT, capture_output=True,
                             env=dict(os.environ, PYTHONIOENCODING='utf-8', PK_SCENARIO_EVIDENCE_DIR=str(HERE)), timeout=900)
    log = HERE / filename
    log.write_bytes(process.stdout + process.stderr)
    results.append({'command':command, 'cwd':str(ROOT), 'exitCode':process.returncode,
                    'log':log.relative_to(ROOT).as_posix(), 'logSha256':hashlib.sha256(log.read_bytes()).hexdigest()})
    print(json.dumps(results[-1]), flush=True)
    if process.returncode:
        raise SystemExit(process.returncode)
files = [ROOT/'crates/core/tests/business_scenarios.rs', ROOT/'scripts/verify-plugins.mjs',
         ROOT/'scripts/package-api-examples.mjs', ROOT/'fixtures/truth-supplements/T12-api.json']
for layout in ['a','b']:
    for capability in ['frontend','api']:
        files.extend(p for p in (ROOT/f'collectors/examples/frontend-backend-{layout}/{capability}').rglob('*') if p.is_file())
report = {'date':'2026-10-28 Asia/Shanghai', 'task':'T12', 'status':'doing',
          'completedSubloop':'TS/Python HTTP and GraphQL API registration with Core execution and negative cases',
          'remaining':['database/source-instance binding', 'DDL/SQLGlot AST', 'composite keys and joins', 'queryImplementation extraction'],
          'commands':results, 'fileHashes':{p.relative_to(ROOT).as_posix():hashlib.sha256(p.read_bytes()).hexdigest() for p in files},
          'testSummaries':re.findall(r'test result:.*', (HERE/'workspace-tests.log').read_text(encoding='utf-8',errors='replace')),
          'productModelRequests':0, 'productionAccuracyClaimed':False}
(HERE/'api-verification-report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
