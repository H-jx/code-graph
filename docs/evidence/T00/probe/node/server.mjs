import { adapter, mcpRequire, native } from './shared.mjs';
const { McpServer } = mcpRequire('@modelcontextprotocol/server');
const { serveStdio } = mcpRequire('@modelcontextprotocol/server/stdio');
serveStdio(() => {
  const server = new McpServer(
    { name: 't00-core-schema-probe', version: '0.0.0' },
    { capabilities: { tools: {} } },
  );
  server.registerTool(
    'probe_validate',
    { inputSchema: adapter(), description: 'Validate with Rust Core' },
    async (input) => {
      const result = JSON.parse(native().validateCollector(JSON.stringify(input)));
      return {
        content: [{ type: 'text', text: JSON.stringify(result) }],
        structuredContent: result,
      };
    },
  );
  return server;
});
