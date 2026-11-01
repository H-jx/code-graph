"""Exercise the installed-shape CLI against independently written local SQL."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
records=[]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def save(name,value): (HERE/name).write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
def run(command,name,expected=(0,2)):
    process=subprocess.run([str(c) for c in command],cwd=ROOT,capture_output=True,timeout=300)
    (HERE/(name+'.stdout.json')).write_bytes(process.stdout)
    (HERE/(name+'.stderr.log')).write_bytes(process.stderr)
    records.append(dict(command=[str(c) for c in command],cwd=str(ROOT),exitCode=process.returncode,expectedExitCodes=list(expected),stdout=name+'.stdout.json',stderr=name+'.stderr.log'))
    assert process.returncode in expected,(name,process.returncode,process.stdout.decode('utf-8',errors='replace'),process.stderr.decode('utf-8',errors='replace'))
    return json.loads(process.stdout)

build=subprocess.run(['cargo','build','-p','project-knowledge-cli','--locked'],cwd=ROOT,capture_output=True,timeout=600)
(HERE/'cli-build.log').write_bytes(build.stdout+build.stderr)
assert build.returncode==0
binary=ROOT/'target/debug'/('project-knowledge.exe' if os.name=='nt' else 'project-knowledge')
binary_hash=sha(binary)
verified=ROOT/'target/verified-cli'/binary_hash/binary.name
verified.parent.mkdir(parents=True,exist_ok=True)
if not verified.exists():shutil.copy2(binary,verified)
assert sha(verified)==binary_hash

with tempfile.TemporaryDirectory(prefix='knowledge-cli-') as temp:
    root=Path(temp);repo=root/'repo';config=repo/'.project-knowledge/project.json'
    config.parent.mkdir(parents=True);(repo/'src').mkdir()
    sql=repo/'src/ledger.sql'
    sql.write_text('-- ledger quantity\nSELECT ordered_qty FROM purchase_line;\n',encoding='utf-8')
    config.write_text(json.dumps(dict(schemaVersion=1,projectId='cli-knowledge',repositories=[dict(id='main',root='..')],selection={})),encoding='utf-8')
    def command(*args):return [verified,*args]
    run(command('index','--config',config,'--base'),'cli-index')
    query=root/'query.json';query.write_text(json.dumps(dict(operation='search',query='ledger',filters=dict(repositoryIds=['main']),pageSize=8)),encoding='utf-8')
    context_file=root/'context.json'
    context=run(command('generate','--config',config,'context','--topic','ledger','--request',query,'--output',context_file),'cli-context')
    assert context==json.loads(context_file.read_text(encoding='utf-8'))
    assert len(json.dumps(context,ensure_ascii=False,separators=(',',':')))<=80000
    citation=max(context['sourceReferences'],key=lambda r:r['endByte']-r['startByte'])
    read_request=root/'read.json';read_request.write_text(json.dumps(dict(operation='readSource',references=[citation])),encoding='utf-8')
    read=run(command('query','--config',config,'--request',read_request),'cli-read-source')
    assert read['data']['reads'][0]['status']=='current'
    assert 'SELECT ordered_qty FROM purchase_line;' in read['data']['reads'][0]['snippet']
    def topic_from(ctx,text):
        citation=max(ctx['sourceReferences'],key=lambda r:r['endByte']-r['startByte'])
        return dict(protocolVersion='1.0',topicId=ctx['topicId'],title='Original local ledger query',snapshot=ctx['snapshot'],inputDigest=ctx['inputDigest'],markdown=text+' [[claim:original_sql]]',claims=[dict(id='original_sql',text=text,basis='observed',origin='human',evidenceRefs=[citation['evidenceId']],gapIds=[])],sourceReferences=ctx['sourceReferences'],gaps=ctx['gaps'],promptHash=ctx['promptHash'],templateHash=ctx['templateHash'])
    artifact=topic_from(context,'The original local SQL selects ordered_qty from purchase_line.')
    topic_file=root/'topic.json';topic_file.write_text(json.dumps(artifact),encoding='utf-8')
    imported=run(command('generate','--config',config,'docs','--input',topic_file),'cli-import')
    assert imported['version']==1
    output=repo/'.agents/skills/ledger'
    generated=run(command('generate','--config',config,'skill','--name','ledger','--output',output),'cli-skill')
    assert generated['freshness']=='fresh'
    directory=Path(generated['directory'])
    manifest=json.loads((directory/'references/index-manifest.json').read_text(encoding='utf-8'))
    for name,expected in manifest['files'].items():assert sha(directory/name)==expected
    assert sha(directory/'references/index-manifest.json')==generated['manifestSha256']
    pointer=json.loads((output/'CURRENT.json').read_text(encoding='utf-8'))
    assert output/pointer['directory']==directory
    saved=HERE/'cli-skill'
    # Only copy the complete generated artifact; never copy the source/index or credentials.
    shutil.copytree(directory,saved,dirs_exist_ok=True)
    snapshot=run(command('status','--config',config,'--check-freshness'),'cli-status')
    assert snapshot['data']['knowledge']['topics'][0]['freshness']=='fresh'
    sql.write_text('-- ledger quantity changed\nSELECT SUM(ordered_qty) FROM purchase_line;\n',encoding='utf-8')
    stale=run(command('status','--config',config,'--check-freshness'),'cli-stale')
    assert stale['data']['knowledge']['topics'][0]['freshness']=='stale'
    rerendered=run(command('generate','--config',config,'skill','--name','ledger','--output',output),'cli-stale-render',expected=(2,))
    assert rerendered['freshness']=='stale'
    run(command('index','--config',config,'--base'),'cli-reindex')
    ctx=run(command('generate','--config',config,'context','--topic','ledger','--request',query),'cli-current-context')
    topic_file.write_text(json.dumps(topic_from(ctx,'The original local SQL aggregates SUM(ordered_qty).')),encoding='utf-8')
    current=run(command('generate','--config',config,'docs','--input',topic_file),'cli-reimport')
    assert current['version']==2
    final=run(command('generate','--config',config,'skill','--name','ledger','--output',output),'cli-current-skill')
    assert final['freshness']=='fresh'
    save('cli-report.json',dict(commands=records,cliArtifactPath=str(verified),cliSha256=binary_hash,topicVersions=[1,2],initialSourceSha256=hashlib.sha256(b'-- ledger quantity\nSELECT ordered_qty FROM purchase_line;\n').hexdigest(),freshnessSequence=['fresh','stale','stale','fresh'],modelsCalled=0,businessDatabaseConnections=0))
print('T17 CLI context/import/skill, references, hashes and stale/reimport acceptance passed')
