"""Recheck frozen real-source citations and available upstream bytes without running its app."""
import hashlib
import json
from pathlib import Path
import subprocess

ROOT=Path(__file__).resolve().parents[1]
provenance=json.loads((ROOT/'fixtures/original/provenance.json').read_text(encoding='utf-8'))
source=Path(provenance['sourceRoot'])
rows=[]
for entry in provenance['files']:
    frozen=ROOT/entry['path']
    assert hashlib.sha256(frozen.read_bytes()).hexdigest()==entry['sha256']
    upstream=source/entry['originalPath']
    current=hashlib.sha256(upstream.read_bytes()).hexdigest() if upstream.exists() else None
    rows.append(dict(path=entry['originalPath'],frozenSha256=entry['sha256'],currentSha256=current,matchesFrozen=current==entry['sha256']))
facts=[]
for name in ['scenario1','scenario2']:
    truth=json.loads((ROOT/f'fixtures/truth/{name}.json').read_text(encoding='utf-8'))
    for fact in truth['facts']:
        for evidence in fact['evidence']:
            lines=(ROOT/evidence['path']).read_text(encoding='utf-8').splitlines()
            snippet='\n'.join(lines[evidence['startLine']-1:evidence['endLine']])
            assert evidence['text'] in snippet,(fact['id'],evidence['path'])
        facts.append(dict(scenario=name,id=fact['id'],citations=len(fact['evidence'])))
head=None
if source.exists():
    git=subprocess.run(['git','-C',str(source),'rev-parse','HEAD'],capture_output=True,text=True)
    if git.returncode==0: head=git.stdout.strip()
report=dict(passed=True,method='Direct frozen original bytes and exact line citations, independent of collectors/models',reviewer='Codex; no human sign-off claimed',sourceAvailable=source.exists(),sourceHead=head,frozenRevision=provenance['revision'],files=rows,facts=facts,currentFilesMatchingFrozen=sum(row['matchesFrozen'] for row in rows),limitations=['Source-declared facts only; no claim of production business correctness','Frontend/backend are separate source areas within the same supplied repository; no second authorized repository supplied','A/B layouts and numerical audit remain separate smoke/audit populations'])
out=ROOT/'docs/evidence/T22/source-review.json'
out.parent.mkdir(parents=True,exist_ok=True)
out.write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps(dict(passed=True,files=len(rows),facts=len(facts),sourceAvailable=source.exists(),currentFilesMatchingFrozen=report['currentFilesMatchingFrozen']),ensure_ascii=False))
