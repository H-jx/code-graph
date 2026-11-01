"""T11 full acceptance/quality checks with exact commands and artifact hashes."""
import hashlib
import json
import os
import pathlib
import re
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[3]
HERE = pathlib.Path(__file__).resolve().parent
commands = [
    ('packaging.log', ['node', 'scripts/package-example-collectors.mjs', '--check']),
    ('plugins.log', ['node', 'scripts/verify-plugins.mjs', '--capabilities', 'routes,frontend,comments,modules,entrypoints']),
    ('workspace-tests.log', ['cargo', 'test', '--workspace', '--locked', '-j', '1', '--', '--test-threads=1']),
    ('clippy.log', ['cargo', 'clippy', '--workspace', '--all-targets', '--locked', '-j', '1', '--', '-D', 'warnings']),
    ('fmt.log', ['cargo', 'fmt', '--all', '--', '--check']),
    ('contracts.log', ['cargo', 'run', '-p', 'xtask', '--locked', '--', 'export-contracts', '--check']),
    ('types.log', ['node', 'scripts/generate-types.mjs', '--check']),
    ('plan.log', ['node', 'scripts/verify-plan.mjs', '--plan', '../plan/NEW-PROJECT-LONG-TASK.md']),
    ('truth.log', ['python', 'scripts/verify-truth.py']),
]
results = []
for filename, command in commands:
    print('Running ' + ' '.join(command), flush=True)
    process = subprocess.run(command, cwd=ROOT, capture_output=True,
                             env=dict(os.environ, PYTHONIOENCODING='utf-8', PK_SCENARIO_EVIDENCE_DIR=str(HERE)))
    log = HERE / filename
    log.write_bytes(process.stdout + process.stderr)
    results.append({'command': command, 'cwd': str(ROOT), 'exitCode': process.returncode,
                    'log': log.relative_to(ROOT).as_posix(),
                    'logSha256': hashlib.sha256(log.read_bytes()).hexdigest()})
    print(json.dumps(results[-1]), flush=True)
    if process.returncode:
        raise SystemExit(process.returncode)
files = [ROOT / 'collectors/templates/node/lib/frontend-ast.mjs', ROOT / 'scripts/package-example-collectors.mjs',
         ROOT / 'scripts/verify-plugins.mjs', ROOT / 'crates/core/tests/business_scenarios.rs',
         ROOT / 'crates/core/src/collect/staging.rs']
for layout in ['a','b']:
    files.extend(p for p in (ROOT / f'collectors/examples/frontend-backend-{layout}/frontend').rglob('*') if p.is_file())
manifest = json.loads((ROOT / 'fixtures/truth/manifest.json').read_text(encoding='utf-8'))
report = {'date':'2026-10-28 Asia/Shanghai','task':'T11','truthHash':manifest['truthHash'],
          'commands':results,'fileHashes':{p.relative_to(ROOT).as_posix():hashlib.sha256(p.read_bytes()).hexdigest() for p in files},
          'acceptance':{'layouts':2,'expectedEntities':26,'expectedRelations':20,'forbiddenHits':0,
                        'classification':'source-reviewed synthetic smoke; not production accuracy'},
          'testSummaries':re.findall(r'test result:.*', (HERE/'workspace-tests.log').read_text(encoding='utf-8',errors='replace'))}
(HERE/'verification-report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
