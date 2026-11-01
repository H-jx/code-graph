// 必须由 ESM 的真实模块解析器从准备缓存导入，NODE_PATH 不足以满足该条件。
import isNumber from 'is-number';
if (
  !isNumber('123.45') ||
  process.env.OPENAI_API_KEY ||
  process.env.ANTHROPIC_API_KEY ||
  process.env.NODE_PATH
)
  process.exit(41);
let text = '';
for await (const chunk of process.stdin) text += chunk;
const input = JSON.parse(text);
process.stdout.write(
  JSON.stringify({
    protocolVersion: '1.0',
    runId: input.runId,
    stageId: input.stageId,
    inputSnapshotId: input.inputSnapshotId,
    entities: [],
    relations: [],
    evidence: [],
    dependencies: [],
    coverage: [],
    gaps: [],
  }),
);
