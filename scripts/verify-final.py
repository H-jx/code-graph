"""Final Windows/local gates. Each attempt is retained; a failure never yields a pass report."""
import argparse
import hashlib
import json
import os
import platform
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'docs/evidence/T22'

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def save(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')

def history():
    merged={}
    for path in OUT.glob('commands*.json'):
        for record in json.loads(path.read_text(encoding='utf-8')):
            merged[(record['name'],record['log'])]=record
    return sorted(merged.values(),key=lambda r:(r['name'],int(re.search(r'-(\d+)\.log$',r['log'])[1])))

def pnpm(script, *args):
    if os.name == 'nt':
        quote=lambda value: "'"+value.replace("'","''")+"'"
        return ['powershell', '-NoProfile', '-Command', 'pnpm run '+quote(script)+(' -- '+' '.join(map(quote,args)) if args else '')]
    return ['pnpm', 'run', script, *(['--', *args] if args else [])]

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--group',choices=['build','tests','node','plugins','static','scenarios','finalize','all'],default='all')
    args=parser.parse_args()
    OUT.mkdir(parents=True,exist_ok=True)
    records_path=OUT/f'commands-{args.group}.json'
    records=json.loads(records_path.read_text(encoding='utf-8')) if records_path.exists() else []
    env=dict(os.environ,PYTHONIOENCODING='utf-8',PK_MCP_EVIDENCE_DIR=str(ROOT/'docs/evidence/T21'),PK_SDK_EVIDENCE_DIR=str(OUT/'sdk'))
    groups={
      'build':[('native',pnpm('build:native')),('mcp',pnpm('build:mcp')),('cli',['cargo','build','-p','project-knowledge-cli','--locked','-j','1'])],
      'tests':[('workspace',['cargo','test','--workspace','--locked','-j','1','--','--test-threads=1']),('sdk',pnpm('test:sdk')),('mcp-tests',pnpm('test:mcp')),('plugins',pnpm('test:plugins','--capabilities','routes,frontend,comments,modules,entrypoints,api,database,link,workflow,report'))],
      'static':[('fmt',['cargo','fmt','--all','--','--check']),('clippy',['cargo','clippy','--workspace','--all-targets','--locked','-j','1','--','-D','warnings']),('contracts',pnpm('check:contracts')),('typecheck',pnpm('typecheck')),('core-deps',['node','scripts/verify-core-dependencies.mjs']),('truth',[sys.executable,'scripts/verify-truth.py']),('source-review',[sys.executable,'scripts/verify-source-review.py']),('plan',['node','scripts/verify-plan.mjs']),('source-audit',['node','scripts/audit-source.mjs','--output',str(OUT/'source-audit.json')]),('diff',['git','diff','--check'])],
      'scenarios':[('scenarios',pnpm('verify:scenarios','--scenario','all','--transport','all'))],
    }
    aliases={'plugins':[groups['tests'][-1]],'node':groups['tests'][1:3]}
    selected=groups if args.group=='all' else {args.group:groups.get(args.group,aliases.get(args.group,[]))}
    for group,commands in selected.items():
      for name,command in commands:
        previous=[int(re.search(r'-(\d+)\.log$',p.name)[1]) for p in OUT.glob(f'{name}-*.log')]
        attempt=max(previous,default=0)+1
        evidence_directory=None
        if name=='scenarios':
          evidence_directory=f'docs/evidence/T22/scenarios/run-{attempt:02}'
          command=pnpm('verify:scenarios','--scenario','all','--transport','all','--evidence',evidence_directory)
        log=OUT/f'{name}-{attempt:02}.log'
        print('Running '+' '.join(command),flush=True)
        started=time.monotonic()
        try:
          process=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,timeout=2400)
          output,code=process.stdout+process.stderr,process.returncode
        except subprocess.TimeoutExpired as error:
          output,code=(error.stdout or b'')+(error.stderr or b'')+b'\nTIMEOUT; not passed\n',-1
        log.write_bytes(output)
        records.append(dict(name=name,group=group,command=command,evidenceDirectory=evidence_directory,exitCode=code,elapsedSeconds=round(time.monotonic()-started,3),log=log.name,sha256=sha(log)))
        save(records_path,records)
        print(f'{name}: exit={code}; {log.name}',flush=True)
        if code:
          print(output[-6000:].decode('utf-8',errors='replace'),flush=True)
          return 1
    if args.group not in ('finalize','all'): return 0
    records=history()
    save(OUT/'commands.json',records)
    latest={r['name']:r for r in records}
    for commands in groups.values():
      for name,_ in commands:
        record=latest[name]
        assert record['exitCode']==0 and sha(OUT/record['log'])==record['sha256'],name
    workspace=(OUT/latest['workspace']['log']).read_text(encoding='utf-8')
    assert not re.search(r'\b[1-9]\d* failed;',workspace)
    tests=sum(map(int,re.findall(r'test result: ok\. (\d+) passed;',workspace)))
    audit=json.loads((OUT/'source-audit.json').read_text(encoding='utf-8'))
    assert not audit['zeroByteSources']
    comparisons=list((ROOT/latest['scenarios']['evidenceDirectory']/'transports').glob('*.json'))
    assert len(comparisons)>=6
    for file in comparisons: assert json.loads(file.read_text(encoding='utf-8'))['passed']
    for name in ['query-resources.json','maintenance-cancellation.json']:
      assert json.loads((ROOT/'docs/evidence/T21'/name).read_text(encoding='utf-8'))['passed']
    for name,digest in audit['fileHashes'].items(): assert sha(ROOT/name)==digest,name
    save(OUT/'verification-report.json',dict(passed=True,platform=sys.platform,architecture=platform.machine(),workspaceRustTests=tests,transportComparisons=len(comparisons),commands=list(latest.values()),sourceHashes=audit['fileHashes'],truthHash=json.loads((ROOT/'fixtures/truth/manifest.json').read_text(encoding='utf-8'))['truthHash'],externalModelRequests=0,businessDatabaseConnections=0,productionAccuracyClaimed=False,limitations=['Only this host was tested in this report','Original synes frontend/backend citations rechecked separately; no second live repository or production accuracy claim','No public publication']))
    print(f'Final gates passed: Rust {tests}; CLI/SDK/MCP comparisons {len(comparisons)}',flush=True)
    return 0

if __name__=='__main__':
    sys.stdout.reconfigure(encoding='utf-8')
    sys.stderr.reconfigure(encoding='utf-8')
    sys.exit(main())
