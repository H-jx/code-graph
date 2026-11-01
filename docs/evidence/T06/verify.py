"""Reproduce T06 verification and record bounded output plus exact source hashes."""
import hashlib
import json
import os
import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[3]
HERE = pathlib.Path(__file__).resolve().parent
commands = [
    ['python', 'scripts/verify-truth.py'],
    ['cargo', 'test', '-p', 'project-knowledge-core', '--test', 'business_scenarios', 'truth_', '--locked', '-j', '1'],
]
results = []
for index, command in enumerate(commands):
    process = subprocess.run(command, cwd=ROOT, capture_output=True, env=dict(os.environ, PYTHONIOENCODING='utf-8'))
    log = HERE / ['arithmetic-oracle.json', 'truth-tests.log'][index]
    log.write_bytes(process.stdout + process.stderr)
    results.append({'command': command, 'cwd': str(ROOT), 'exitCode': process.returncode,
                    'log': log.relative_to(ROOT).as_posix(),
                    'logSha256': hashlib.sha256(log.read_bytes()).hexdigest()})
    print(json.dumps(results[-1]), flush=True)
    if process.returncode:
        raise SystemExit(process.returncode)
manifest = json.loads((ROOT / 'fixtures/truth/manifest.json').read_text(encoding='utf-8'))
hashes = {}
for path in ['crates/core/tests/business_scenarios.rs', 'scripts/verify-truth.py', 'crates/core/src/collect/candidate.rs']:
    hashes[path] = hashlib.sha256((ROOT / path).read_bytes()).hexdigest()
(HERE / 'verification-report.json').write_text(json.dumps({
    'date': '2026-10-28 Asia/Shanghai', 'task': 'T06', 'truthHash': manifest['truthHash'],
    'commands': results, 'sourceHashes': hashes, 'productionAccuracyClaimed': False,
}, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
