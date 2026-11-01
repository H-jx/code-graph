#!/usr/bin/env node
import { parseArgs } from 'node:util';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { McpServer } from '@modelcontextprotocol/server';
import { serveStdio } from '@modelcontextprotocol/server/stdio';
import { ProjectKnowledge } from '@project-knowledge/sdk';
import { Session, registerTools } from './tools.js';
import { registerResources } from './resources.js';

export async function start(options: { configPath: string; mode?: 'query' | 'maintenance' }) {
  const session = new Session(resolve(options.configPath), options.mode ?? 'query');
  const capabilities = await ProjectKnowledge.getCapabilities();
  const handle = serveStdio(
    () => {
      const server = new McpServer(
        { name: 'project-knowledge', version: '0.1.0' },
        { capabilities: { tools: {}, resources: {} } },
      );
      registerTools(server, session, capabilities);
      registerResources(server, session, capabilities);
      server.server.onclose = () => {
        void session.close().catch(() => {
          process.stderr.write('MCP cleanup failed\n');
          process.exitCode = 1;
        });
      };
      return server;
    },
    { onerror: () => process.stderr.write('MCP protocol error\n') },
  );
  let closing: Promise<void> | undefined;
  return {
    close: () =>
      (closing ??= (async () => {
        await handle.close();
        await session.close();
      })()),
  };
}

async function main() {
  const { values } = parseArgs({
    options: {
      config: { type: 'string' },
      mode: { type: 'string', default: 'query' },
      help: { type: 'boolean' },
    },
  });
  if (values.help) {
    process.stderr.write(
      'project-knowledge-mcp --config <project.json> [--mode query|maintenance]\n',
    );
    return;
  }
  if (!values.config || !['query', 'maintenance'].includes(values.mode))
    throw new Error('Use --config <project.json> [--mode query|maintenance]');
  const handle = await start({
    configPath: values.config,
    mode: values.mode as 'query' | 'maintenance',
  });
  for (const signal of ['SIGINT', 'SIGTERM'] as const)
    process.once(signal, () => {
      void handle.close().catch(() => {
        process.exitCode = 1;
      });
    });
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main().catch(() => {
    process.stderr.write(
      'MCP startup failed; check --config, --mode and the installed native platform package\n',
    );
    process.exitCode = 1;
  });
}
