// 合成增量生命周期 fixture；不作为独立业务真值。
import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import { writeFileSync } from 'node:fs';
const hash = value => createHash('sha256').update(JSON.stringify(value)).digest('hex');
let text = '';
for await (const chunk of process.stdin) text += chunk;
const input = JSON.parse(text);
const output = { protocolVersion:'1.0', runId:input.runId, stageId:input.stageId,
  inputSnapshotId:input.inputSnapshotId, entities:[], relations:[], evidence:[], dependencies:[], coverage:[], gaps:[] };
const project = input.configSubset.projectId;
const source = input.sources[0];
if (input.sources.some(s => s.content.includes('FAIL'))) process.exit(42);
const hold = input.sources.map(s => s.content.match(/HOLD:(.+)/)?.[1]).find(Boolean);
if (hold) {
  const child = spawn(process.execPath, ['-e','setInterval(() => {},1000)'], {stdio:'ignore'});
  child.once('spawn', () => writeFileSync(hold,JSON.stringify({parent:process.pid,descendant:child.pid})));
  await new Promise(() => {});
}
const assertion = s => {
  const id = `ev_${hash([s.id,s.contentHash,0,1,'binding'])}`;
  output.evidence.push({id,sourceId:s.id,contentHash:s.contentHash,startByte:0,endByte:1,startLine:1,endLine:1,role:'binding'});
  return {producerKey:'fixture',name:s.content.trim(),attributes:{text:s.content.trim()},basis:'observed',origin:'static',evidenceRefs:[id]};
};
if (input.stageId === 'extract') {
  for (const s of input.sources) {
    const route = s.content.match(/ROUTE:(\S+)/)?.[1];
    if (!route) continue;
    const scope = `repo.${s.repositoryId}`;
    output.entities.push({entityId:`ent_${hash([project,'route',scope,'router','history',route])}`,kind:'route',scopeId:scope,
      identity:[['scopeId',scope],['routerId','router'],['mode','history'],['declaredPath',route]].map(([name,value])=>({name,value})),assertion:assertion(s)});
  }
} else if (source) {
  const routes = input.entities.filter(e => e.kind === 'route');
  const file = input.entities.find(e => e.kind === 'file');
  if (routes.length === 1 && file) {
    const fromId=file.entityId, toId=routes[0].entityId;
    output.relations.push({relationId:`rel_${hash([project,'contains',fromId,toId,{}])}`,kind:'contains',fromId,toId,qualifiers:{},resolution:'confirmed',assertion:assertion(source)});
  }
}
process.stdout.write(JSON.stringify(output));
