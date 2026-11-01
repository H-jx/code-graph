"""One current configured-provider query smoke, without persisting requests or credentials."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[3]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
records=[]
def command(name,args,expected=(0,)):
    process=subprocess.run([str(x) for x in args],cwd=ROOT,capture_output=True,timeout=180)
    (HERE/(name+'.stdout.json')).write_bytes(process.stdout)
    (HERE/(name+'.stderr.log')).write_bytes(process.stderr)
    records.append(dict(command=[str(x) for x in args],cwd=str(ROOT),exitCode=process.returncode,stdout=name+'.stdout.json',stderr=name+'.stderr.log'))
    assert process.returncode in expected, (name,process.returncode,'inspect sanitized CLI output')
    return json.loads(process.stdout)

build=subprocess.run(['cargo','build','-p','project-knowledge-cli','--locked'],cwd=ROOT,capture_output=True,timeout=600)
(HERE/'live-build.log').write_bytes(build.stdout+build.stderr)
assert build.returncode==0
binary=ROOT/'target/debug'/('project-knowledge.exe' if os.name=='nt' else 'project-knowledge')
frozen=ROOT/'target/verified-cli'/sha(binary)/binary.name
frozen.parent.mkdir(parents=True,exist_ok=True)
if not frozen.exists():shutil.copy2(binary,frozen)
assert sha(binary)==sha(frozen)
project=HERE/'live-smoke';config=project/'.project-knowledge/project.json'
config.parent.mkdir(parents=True,exist_ok=True)
(project/'sample.ts').write_text('export const querySmoke = 1;\n',encoding='utf-8')
config.write_text(json.dumps(dict(schemaVersion=1,projectId='T18-query-smoke',repositories=[dict(id='smoke',root='..')],selection={})),encoding='utf-8')
env_file=ROOT/'.env'
assert env_file.is_file(),'configured env file is missing'
configure=[frozen,'agent','configure','--config',config,'--env-file',env_file,'--max-turns','4','--max-tool-calls','4','--timeout-seconds','60','--max-input-chars','80000','--max-output-tokens','512']
configured=command('live-configure',configure)
assert configured['keyConfigured'] is True
command('live-index',[frozen,'index','--config',config,'--base'],(0,2))
result=command('real-smoke',[*configure,'--check'])
assert result['status']=='complete',result.get('status')
assert result['smoke'] is True and result['toolCalls']>=1
assert any(e.get('tool')=='status' and e.get('ok') is True for e in result['events'])
report=dict(date='2026-10-28 Asia/Shanghai',scope='T18 real configured endpoint query tool smoke; no plugin generation, no business accuracy claim',provider=result['provider'],model=result['model'],status=result['status'],runId=result['runId'],modelTurns=result['modelTurns'],toolCalls=result['toolCalls'],elapsedMs=result['elapsedMs'],promptHash=result['promptHash'],responseMetadata=[e for e in result['events'] if 'responseId' in e],commands=records,cliArtifactPath=str(frozen),cliSha256=sha(frozen),sourceHashes={p.relative_to(ROOT).as_posix():sha(p) for p in [ROOT/'crates/agent/src/config.rs',ROOT/'crates/agent/src/prompts.rs',ROOT/'crates/agent/src/tools.rs',ROOT/'crates/agent/src/runner.rs',ROOT/'crates/cli/src/commands/agent.rs']})
(HERE/'live-report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({k:report[k] for k in ['provider','model','status','modelTurns','toolCalls','elapsedMs','cliSha256']}),flush=True)
