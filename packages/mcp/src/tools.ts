import {
  ProtocolError,
  INVALID_PARAMS,
  type McpServer,
  type ServerContext,
  type CallToolResult,
  type StandardSchemaWithJSON,
} from '@modelcontextprotocol/server';
import { createHash } from 'node:crypto';
import { ProjectKnowledge, MaintenanceClient, ProjectKnowledgeError } from '@project-knowledge/sdk';
import type { QueryRequest, MaintenanceRequest, Capabilities } from '@project-knowledge/sdk';
import { coreSchema } from './schema-adapter.js';

export const queryTools = {
  index_status: 'status',
  search: 'search',
  locate_feature: 'locateFeature',
  trace_links: 'trace',
  neighbors: 'neighbors',
  get_code_map: 'codeMap',
  read_source: 'readSource',
  locate_page: 'page',
  explain_workflow: 'workflow',
  explain_report: 'report',
} as const;
export const maintenanceTools = {
  export_snapshot: 'exportContext',
  write_candidate: 'submitPlugin',
  discover_plugins: 'discoverPlugins',
  prepare_plugin: 'preparePlugin',
  validate_plugin: 'validatePlugin',
  run_plugin_cases: 'validatePlugin',
  activate_plugin: 'activatePlugin',
  rollback_plugin: 'rollbackPlugin',
  update_index: 'index',
  collect: 'collect',
  export_topic: 'exportContext',
  import_topic: 'importTopic',
  import_conventions: 'importConventions',
  generate_skill: 'generateSkill',
} as const;

/** One lazy SDK connection per stdio session, including ownership of a pending open. */
export class Session {
  #opening: Promise<ProjectKnowledge> | undefined;
  #closing: Promise<void> | undefined;
  constructor(
    readonly configPath: string,
    readonly mode: 'query' | 'maintenance',
  ) {}
  open(): Promise<ProjectKnowledge> {
    if (this.#closing)
      return Promise.reject(
        new ProjectKnowledgeError({ code: 'CLOSED', message: 'Session closed', retryable: false }),
      );
    this.#opening ??= (
      this.mode === 'maintenance'
        ? ProjectKnowledge.openMaintenance({ configPath: this.configPath })
        : ProjectKnowledge.open({ configPath: this.configPath })
    ).catch((error) => {
      this.#opening = undefined;
      throw error;
    });
    return this.#opening;
  }
  async execute(
    request: QueryRequest | MaintenanceRequest,
    write: boolean,
    ctx: ServerContext,
  ): Promise<unknown> {
    if (write && this.mode !== 'maintenance')
      throw new ProjectKnowledgeError({
        code: 'READ_ONLY',
        message: 'Maintenance mode required',
        retryable: false,
      });
    const client = await this.open();
    const options = {
      signal: ctx.mcpReq.signal,
      requestId: `mcp-${createHash('sha256')
        .update(JSON.stringify([typeof ctx.mcpReq.id, ctx.mcpReq.id]))
        .digest('hex')}`,
    };
    if (write) {
      if (!(client instanceof MaintenanceClient)) throw new Error('Invalid maintenance session');
      return client.maintain(request as MaintenanceRequest, options);
    }
    return client.execute(request as QueryRequest, options);
  }
  close(): Promise<void> {
    this.#closing ??= (async () => {
      const client = await this.#opening?.catch(() => undefined);
      await client?.close();
    })();
    return this.#closing;
  }
}

export function errorEnvelope(error: unknown) {
  if (error instanceof ProjectKnowledgeError)
    return {
      protocolVersion: '1.0',
      error: {
        code: error.code,
        message: error.message,
        retryable: error.retryable,
        issues: error.issues,
        ...(error.runId ? { runId: error.runId } : {}),
        ...(error.phase ? { phase: error.phase } : {}),
      },
    };
  return {
    protocolVersion: '1.0',
    error: {
      code: 'INTERNAL_ERROR',
      message: 'MCP operation failed',
      retryable: false,
      issues: [],
    },
  };
}
export function result(value: unknown, isError = false): CallToolResult {
  const structuredContent =
    value !== null && typeof value === 'object' && !Array.isArray(value)
      ? (value as Record<string, unknown>)
      : { data: value };
  return {
    content: [{ type: 'text', text: JSON.stringify(structuredContent) }],
    structuredContent,
    ...(isError ? { isError: true } : {}),
  };
}

export function registerTools(server: McpServer, session: Session, capabilities: Capabilities) {
  const handlers = new Map<
    string,
    {
      schema: StandardSchemaWithJSON<Record<string, unknown>>;
      run: (args: Record<string, unknown>, ctx: ServerContext) => Promise<CallToolResult>;
    }
  >();
  for (const [write, mapping] of [
    [false, queryTools],
    [true, maintenanceTools],
  ] as const) {
    if (write && session.mode !== 'maintenance') continue;
    for (const [name, operation] of Object.entries(mapping)) {
      const schema = coreSchema(capabilities.schema, operation, write);
      const run = async (args: Record<string, unknown>, ctx: ServerContext) => {
        try {
          return result(
            await session.execute(
              { ...args, operation } as QueryRequest | MaintenanceRequest,
              write,
              ctx,
            ),
          );
        } catch (error) {
          return result(errorEnvelope(error), true);
        }
      };
      handlers.set(name, { schema, run });
      server.registerTool(
        name,
        {
          description: `Core ${operation}. ${write ? 'Explicit maintenance; may update project state.' : 'Read-only; inspect completeness and gaps, including partial results.'}`,
          inputSchema: schema,
          annotations: {
            readOnlyHint: !write,
            destructiveHint: write,
            idempotentHint: !write,
            openWorldHint: write && operation === 'preparePlugin',
          },
        },
        run,
      );
    }
  }
  // MCP 2.4's high-level handler turns schema failures into tool execution errors.
  // Preserve protocol InvalidParams for invalid arguments with the public handler API.
  server.server.setRequestHandler('tools/call', async (request, ctx) => {
    const handler = handlers.get(request.params.name);
    if (!handler)
      throw new ProtocolError(INVALID_PARAMS, 'Tool is not available in this server mode');
    const parsed = await handler.schema['~standard'].validate(request.params.arguments ?? {});
    if (parsed.issues)
      throw new ProtocolError(INVALID_PARAMS, 'Invalid tool arguments', { issues: parsed.issues });
    return handler.run(parsed.value, ctx);
  });
}
