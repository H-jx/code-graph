// 用真实编译器检查生成类型，不另写协议 interface。
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import ts from 'typescript';

const directory = mkdtempSync(join(tmpdir(), 'pk-contract-types-'));
try {
  const path = join(directory, 'contract-boundaries.ts');
  const modulePath = resolve('packages/sdk/src/generated/contract.ts').replaceAll('\\', '/');
  writeFileSync(
    path,
    `
import type {IdentitySlot,CollectorOutput,ManifestExtensions,ProjectKnowledgeContract,RelationRef,MaintenanceRequest} from ${JSON.stringify(modulePath)};
const absent: IdentitySlot = {name:'catalog'};
const empty: IdentitySlot = {name:'catalog',value:''};
const nested: IdentitySlot = {name:'key',value:[['中文😀',1]]};
// @ts-expect-error null is not absence
const nullSlot: IdentitySlot = {name:'catalog',value:null};
// @ts-expect-error boolean is not an identity value
const boolSlot: IdentitySlot = {name:'catalog',value:true};
// @ts-expect-error unknown public fields are forbidden
const extra: IdentitySlot = {name:'catalog',unexpected:1};
const query: ProjectKnowledgeContract = {operation:'search',query:'采购',budget:null};
const neighbors: ProjectKnowledgeContract = {operation:'neighbors',root:{entityId:'ent_1',snapshot:{indexId:'i',generation:'1'}},direction:'both',pageSize:5};
// @ts-expect-error a BFS does not accept a resumable adjacency cursor
const bfsCursor: ProjectKnowledgeContract = {operation:'trace',roots:[],direction:'out',continuation:'cursor'};
// @ts-expect-error required tagged-union payload is missing
const badQuery: ProjectKnowledgeContract = {operation:'search'};
// @ts-expect-error unrecognised union tag
const badTag: ProjectKnowledgeContract = {operation:'unknown'};
const output: CollectorOutput = {protocolVersion:'1.0',runId:'run',stageId:'extract',inputSnapshotId:'input',entities:[],relations:[],evidence:[],dependencies:[],coverage:[],gaps:[]};
// @ts-expect-error required output arrays may not be omitted
const missing: CollectorOutput = {protocolVersion:'1.0',runId:'run',stageId:'extract',inputSnapshotId:'input'};
// @ts-expect-error declarations cannot override public basis
const override: ManifestExtensions = {entities:[{name:'x.plugin.invoice',identityComponents:[],attributesSchema:{},basis:'observed'}]};
const reference: RelationRef = {snapshot:{indexId:'i',generation:'1'},relationId:'rel_1'};
const maintenance: MaintenanceRequest = {operation:'validatePlugin',candidateIds:['c']};
// @ts-expect-error maintenance fields use the same camelCase Rust contract
const wrongMaintenanceField: MaintenanceRequest = {operation:'validatePlugin',candidate_ids:['c']};
// @ts-expect-error index mode is an explicit closed union
const wrongIndexMode: MaintenanceRequest = {operation:'index',mode:'auto'};
// @ts-expect-error maintenance is not a query request
const queryCannotMaintain: ProjectKnowledgeContract = maintenance;
`,
  );
  const program = ts.createProgram([path], {
    strict: true,
    noEmit: true,
    skipLibCheck: true,
    types: [],
    target: ts.ScriptTarget.ES2022,
    module: ts.ModuleKind.NodeNext,
    moduleResolution: ts.ModuleResolutionKind.NodeNext,
  });
  const diagnostics = ts.getPreEmitDiagnostics(program);
  assert.equal(
    diagnostics.length,
    0,
    ts.formatDiagnosticsWithColorAndContext(diagnostics, {
      getCurrentDirectory: () => process.cwd(),
      getCanonicalFileName: (file) => file,
      getNewLine: () => '\n',
    }),
  );
  console.log(
    JSON.stringify({
      ok: true,
      typescript: ts.version,
      positiveCases: 8,
      negativeCases: 9,
      source: 'Rust generated contract.ts',
    }),
  );
} finally {
  rmSync(directory, { recursive: true, force: true });
}
