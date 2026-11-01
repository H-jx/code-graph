"""Final T18 gates; real endpoint smoke is run once separately and hash-checked here."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[3]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
env=dict(os.environ,PYTHONIOENCODING='utf-8',PK_AGENT_EVIDENCE_DIR=str(HERE))
commands=[
    ('agent-final.log',['cargo','test','-p','project-knowledge-agent','--test','agent_lifecycle','--locked','-j','1','--','--test-threads=1']),
    ('indexing-final.log',['cargo','test','-p','project-knowledge-core','--test','indexing','--locked','-j','1','--','--test-threads=1']),
    ('cli-final.log',['cargo','test','-p','project-knowledge-cli','--locked','-j','1','--','--test-threads=1']),
    ('core-only-cli.log',['cargo','test','-p','project-knowledge-cli','--no-default-features','--test','agent','--locked','-j','1','--','--test-threads=1']),
    ('clippy.log',['cargo','clippy','--workspace','--all-targets','--locked','-j','1','--','-D','warnings']),
    ('fmt.log',['cargo','fmt','--all','--','--check']),
    ('contracts.log',['cargo','run','-p','xtask','--locked','--','export-contracts','--check']),
    ('types.log',['node','scripts/generate-types.mjs','--check']),
    ('type-boundaries.log',['node','scripts/verify-contract-types.mjs']),
    ('typecheck.log',['powershell','-NoProfile','-Command','pnpm run typecheck'] if os.name=='nt' else ['pnpm','run','typecheck']),
    ('core-dependencies.log',['node','scripts/verify-core-dependencies.mjs']),
    ('truth.log',[sys.executable,'scripts/verify-truth.py']),
    ('source-audit.log',['node','scripts/audit-source.mjs','--output',str(HERE/'source-audit.json')]),
    ('plan.log',['node','scripts/verify-plan.mjs','--plan','../plan/NEW-PROJECT-LONG-TASK.md']),
    ('diff.log',['git','diff','--check']),
    ('default-cli-build.log',['cargo','build','-p','project-knowledge-cli','--locked']),
]
records=[]
for name,command in commands:
    print('Running '+' '.join(command),flush=True)
    process=subprocess.run(command,cwd=ROOT,capture_output=True,timeout=900,env=env)
    log=HERE/name;log.write_bytes(process.stdout+process.stderr)
    records.append(dict(command=command,cwd=str(ROOT),exitCode=process.returncode,log=log.relative_to(ROOT).as_posix(),logSha256=sha(log)))
    print(json.dumps(records[-1]),flush=True)
    if process.returncode:raise SystemExit(process.returncode)
smoke=json.loads((HERE/'live-report.json').read_text(encoding='utf-8'))
assert smoke['status']=='complete' and sha(Path(smoke['cliArtifactPath']))==smoke['cliSha256']
for path,expected in smoke['sourceHashes'].items():assert sha(ROOT/path)==expected,path
default_cli=ROOT/'target/debug'/('project-knowledge.exe' if os.name=='nt' else 'project-knowledge')
probe=subprocess.run([str(default_cli),'agent','configure','--config',str(HERE/'live-smoke/.project-knowledge/project.json'),'--env-file',str(ROOT/'.env')],cwd=ROOT,capture_output=True,timeout=30,env=env)
(HERE/'default-cli-probe.stdout.json').write_bytes(probe.stdout);(HERE/'default-cli-probe.stderr.log').write_bytes(probe.stderr)
assert probe.returncode==0
configured=json.loads(probe.stdout);assert configured['status']=='configured'
assert configured['agent']['provider']==smoke['provider'] and configured['agent']['model']==smoke['model']
summaries={name:re.findall(r'test result:.*',(HERE/name).read_text(encoding='utf-8',errors='replace')) for name in ['agent-final.log','indexing-final.log','cli-final.log','core-only-cli.log']}
for name,rows in summaries.items():assert any(re.search(r'\b[1-9]\d* passed',r) for r in rows),name
counts={name:sum(int(n) for n in re.findall(r'test result: ok\. (\d+) passed','\n'.join(rows))) for name,rows in summaries.items()}
agent_log=(HERE/'agent-final.log').read_text(encoding='utf-8')
config_count=len(re.findall(r'^test config_.* \.\.\. ok$',agent_log,re.M));tool_count=len(re.findall(r'^test tools_.* \.\.\. ok$',agent_log,re.M))
knowledge_count=len(re.findall(r'^test knowledge::knowledge_.* \.\.\. ok$',(HERE/'indexing-final.log').read_text(encoding='utf-8'),re.M))
assert config_count>0 and tool_count>0 and knowledge_count>0
files=[ROOT/p for p in ['crates/agent/src/config.rs','crates/agent/src/prompts.rs','crates/agent/src/tools.rs','crates/agent/src/runner.rs','crates/agent/tests/agent_lifecycle.rs','crates/cli/src/commands/agent.rs','crates/core/src/knowledge/mod.rs','crates/core/src/knowledge/context.rs','crates/core/src/knowledge/documents.rs','crates/core/src/knowledge/skill.rs','crates/core/tests/support/knowledge.rs','docs/agent-usage.md']]
files.extend((ROOT/'prompts').glob('*.md'));files.extend([HERE/'verify.py',HERE/'live_smoke.py'])
files.extend(ROOT/p for p in ['Cargo.toml','Cargo.lock','rust-toolchain.toml'])
files.append(ROOT/'crates/core/src/storage/knowledge.rs')
snapshots=[p for p in HERE.rglob('*.json') if p.is_file() and '.project-knowledge' not in p.parts and p.name!='verification-report.json']
report=dict(date='2026-10-28 Asia/Shanghai',task='T18',commands=records,testSummaries=summaries,agentTests=16,indexingTests=56,cliTests=6,coreOnlyCliVariantTests=1,configTests=3,toolTests=8,knowledgeCoreTests=7,realQuerySmoke=smoke,sourceHashes={p.relative_to(ROOT).as_posix():sha(p) for p in files},snapshotHashes={p.relative_to(HERE).as_posix():sha(p) for p in snapshots},businessDatabaseConnections=0,productionAccuracyClaimed=False,platform='Windows x64',limitations=['T19 independent business/knowledge lifecycle and recovery acceptance remains open','Real endpoint check is one query tool smoke, not a claim about every provider/model or business scenario','Core-only feature disabled path is verified explicitly; native/SDK/MCP/package stages remain their own tasks'])
report.update(agentTests=counts['agent-final.log'],indexingTests=counts['indexing-final.log'],cliTests=counts['cli-final.log'],coreOnlyCliVariantTests=counts['core-only-cli.log'],configTests=config_count,toolTests=tool_count,knowledgeCoreTests=knowledge_count)
report['toolVersions']={name:subprocess.check_output(command,cwd=ROOT,env=env).decode('utf-8').strip() for name,command in [('rustc',['rustc','--version']),('cargo',['cargo','--version']),('node',['node','--version']),('python',[sys.executable,'--version'])]}
report['defaultCli']={'path':str(default_cli),'sha256':sha(default_cli),'agentFeatureEnabled':True,'probeExitCode':probe.returncode,'probeModelRequests':0}
report['realSmokeRuns']=[json.loads(p.read_text(encoding='utf-8')) for p in HERE.rglob('live-report.json')]
report['externalModelRequests']=sum(r['modelTurns'] for r in report['realSmokeRuns'])
(HERE/'verification-report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('T18 final verification passed',flush=True)
