import assert from 'node:assert/strict';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { mcpRequire, native, valid } from './shared.mjs';
import ts from 'typescript';

test('TypeScript 6.0 Compiler API exposes UTF-16 locations convertible to UTF-8', () => {
  assert.equal(ts.version, '6.0.3');
  const text = 'const 中文 = "😀"; const target = 中文;';
  const source = ts.createSourceFile(
    '位置.ts',
    text,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TS,
  );
  assert.equal(source.parseDiagnostics.length, 0);
  const declaration = source.statements[1].declarationList.declarations[0];
  const start = declaration.getStart(source);
  assert.equal(start, text.indexOf('target'));
  const byte = Buffer.byteLength(text.slice(0, start), 'utf8');
  assert.notEqual(byte, start);
  assert.equal(
    Buffer.from(text)
      .subarray(byte, byte + 6)
      .toString('utf8'),
    'target',
  );
});

test('napi v3 loads and returns Core Schema and exact validation issues', () => {
  const addon = native();
  assert.deepEqual(JSON.parse(addon.validateCollector(JSON.stringify(valid))), { valid: true });
  const { gaps, ...invalid } = valid;
  const error = JSON.parse(addon.validateCollector(JSON.stringify(invalid))).error;
  assert.equal(error.code, 'SCHEMA_INVALID');
  assert.equal(error.issues[0].instancePath, '/gaps');
  assert.ok(JSON.parse(addon.collectorSchema()).required.includes('gaps'));
});

for (const mode of ['legacy', { pin: '2026-07-28' }]) {
  test(
    `MCP 2.4.0 stdio ${JSON.stringify(mode)} lists Rust Schema and calls Core`,
    { timeout: 15000 },
    async () => {
      const { Client } = mcpRequire('@modelcontextprotocol/client');
      const { StdioClientTransport } = mcpRequire('@modelcontextprotocol/client/stdio');
      const transport = new StdioClientTransport({
        command: process.execPath,
        args: [fileURLToPath(new URL('./server.mjs', import.meta.url))],
        stderr: 'pipe',
      });
      const client = new Client(
        { name: 't00-probe-client', version: '0.0.0' },
        { versionNegotiation: { mode } },
      );
      try {
        await client.connect(transport);
        const protocol = client.getNegotiatedProtocolVersion();
        assert.ok(protocol);
        if (mode !== 'legacy') assert.equal(protocol, '2026-07-28');
        else assert.equal(client.getProtocolEra(), 'legacy');
        const { tools } = await client.listTools();
        assert.equal(tools.length, 1);
        assert.deepEqual(tools[0].inputSchema, JSON.parse(native().collectorSchema()));
        const result = await client.callTool({ name: 'probe_validate', arguments: valid });
        assert.deepEqual(result.structuredContent, { valid: true });
        const { gaps, ...invalid } = valid;
        let refused = false;
        try {
          const result = await client.callTool({ name: 'probe_validate', arguments: invalid });
          assert.equal(result.isError, true);
          assert.match(JSON.stringify(result), /SCHEMA_INVALID/);
          assert.match(JSON.stringify(result), /\/gaps/);
          refused = true;
        } catch (error) {
          assert.match(error.message, /SCHEMA_INVALID/);
          assert.match(error.message, /\/gaps/);
          refused = true;
        }
        assert.ok(refused);
        console.log(
          JSON.stringify({
            sdk: '2.4.0',
            protocol,
            era: client.getProtocolEra(),
            list: 1,
            calls: 2,
          }),
        );
      } finally {
        await client.close();
        await transport.close();
      }
    },
  );
}
