import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';

export const root = fileURLToPath(new URL('../../../../../', import.meta.url));
export const mcpRequire = createRequire(resolve(root, 'packages/mcp/package.json'));
let binding;
export function native() {
  if (!binding) {
    const filename =
      process.platform === 'win32'
        ? 't00_native_probe.dll'
        : process.platform === 'darwin'
          ? 'libt00_native_probe.dylib'
          : 'libt00_native_probe.so';
    const module = { exports: {} };
    process.dlopen(module, resolve(root, 'target/debug', filename));
    binding = module.exports;
  }
  return binding;
}
export function adapter() {
  const schema = JSON.parse(native().collectorSchema());
  return {
    '~standard': {
      version: 1,
      vendor: 'project-knowledge-core-probe',
      validate(value) {
        const report = JSON.parse(native().validateCollector(JSON.stringify(value)));
        return report.valid
          ? { value }
          : {
              issues: report.error.issues.map((issue) => ({
                message: JSON.stringify(issue),
                path: issue.instancePath.split('/').slice(1),
              })),
            };
      },
      jsonSchema: { input: () => schema, output: () => schema },
    },
  };
}
export const valid = {
  protocolVersion: '1.0',
  runId: 'probe',
  stageId: 'extract',
  inputSnapshotId: 'probe-input',
  entities: [],
  relations: [],
  evidence: [],
  dependencies: [],
  coverage: [],
  gaps: [],
};
