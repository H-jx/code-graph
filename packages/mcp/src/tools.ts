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

/**
 * MCP-facing default query timeout. The core contract freezes `budget.timeoutMs`'s default
 * at 2s (plan §3.7), so a caller that omits `budget` silently gets `partial` empty results
 * the moment the 2s query deadline trips — easy to misread as "the index has no data".
 * The MCP layer therefore injects a generous deadline (equal to the server hard cap) for
 * read-only query tools whenever the caller did not set `budget.timeoutMs`. An explicit
 * caller value — including `0` — always wins. Only the operations that actually carry a
 * `budget` field are touched: `status` and `readSource` are exempt both here and in the
 * core deadline clamp (see knowledge::query), and maintenance tools are left untouched.
 */
export const DEFAULT_QUERY_TIMEOUT_MS = 30_000;

const BOUNDED_OPERATIONS = new Set([
  'search',
  'locateFeature',
  'trace',
  'neighbors',
  'codeMap',
  'page',
  'workflow',
  'report',
]);

export function withDefaultQueryTimeout<T extends Record<string, unknown>>(
  request: T,
  write: boolean,
): T {
  if (write || !BOUNDED_OPERATIONS.has(String(request['operation']))) return request;
  const budget = request['budget'] as Record<string, unknown> | undefined;
  if (budget?.['timeoutMs'] != null) return request;
  return { ...request, budget: { ...budget, timeoutMs: DEFAULT_QUERY_TIMEOUT_MS } };
}

/**
 * Turns an otherwise *silent* degradation into a visible header. Two core behaviours return
 * near-empty results without an obvious reason — a newcomers reads as "the index has no data":
 *  • the query deadline tripped (data emptied or trimmed, `reasons` contains "deadline reached");
 *  • a stale index, where the core deliberately empties query results (freshness == "stale").
 * Ordinary pagination also returns `status: partial` (with a continuation and a "page boundary"
 * reason); that is NOT a degradation, so we deliberately key only on the two markers above to
 * avoid crying wolf. `stale` is only reported for operations whose results the core actually
 * empties (see query::execute), so `status`/`readSource` never get a misleading "emptied" note.
 */
export function degradedHint(value: unknown, operation: string): string | undefined {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) return undefined;
  const record = value as Record<string, unknown>;
  if (record['error']) return undefined; // error envelope, handled by isError
  const completeness = record['completeness'] as Record<string, unknown> | undefined;
  const reasons = Array.isArray(completeness?.['reasons'])
    ? (completeness!['reasons'] as unknown[]).map(String)
    : [];
  if (reasons.some((reason) => reason.toLowerCase().includes('deadline reached')))
    return (
      'results were truncated: the query did not finish within budget.timeoutMs ' +
      '(the MCP layer already raises the core 2s default to 30s when you omit it). ' +
      'Raise budget.timeoutMs further, or narrow the query with filters/maxDepth.'
    );
  if (completeness?.['freshness'] === 'stale' && BOUNDED_OPERATIONS.has(operation))
    return (
      'results were emptied: the index is stale (freshness=stale) and the core ' +
      'safely clears query data. Rebuild with the index tool, or pass ' +
      'filters.freshnessPolicy="allowStale" to read historical assertions.'
    );
  return undefined;
}

/** Human/agent-facing tool description, advertising the budget defaults for bounded reads. */
export function toolDescription(operation: string, write: boolean): string {
  if (write) return `Core ${operation}. Explicit maintenance; may update project state.`;
  if (!BOUNDED_OPERATIONS.has(operation))
    return `Core ${operation}. Read-only; inspect completeness and gaps, including partial results.`;
  // Mirrors core::contract::query::default_budget; keep in sync if those constants change.
  return (
    `Core ${operation}. Read-only; inspect completeness and gaps, including partial results. ` +
    `Omitting "budget" applies safe defaults: timeout 30s (not the core 2s), maxEntities 60, ` +
    `maxRelations 120, maxVisited 2000, maxDepth 8, maxResponseChars 24000, pageSize 20. ` +
    `A timeout or stale result is truncated/emptied and leads with a "⚠️ degraded" note.`
  );
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
export function result(value: unknown, isError = false, hint?: string): CallToolResult {
  const structuredContent =
    value !== null && typeof value === 'object' && !Array.isArray(value)
      ? (value as Record<string, unknown>)
      : { data: value };
  const json = JSON.stringify(structuredContent);
  return {
    // structuredContent stays byte-identical to the core response; the degraded note is a
    // header on the model-facing text only, so structured consumers are unaffected.
    content: [{ type: 'text', text: hint ? `⚠️ degraded: ${hint}\n${json}` : json }],
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
          const value = await session.execute(
            withDefaultQueryTimeout({ ...args, operation }, write) as
              QueryRequest | MaintenanceRequest,
            write,
            ctx,
          );
          return result(value, false, degradedHint(value, operation));
        } catch (error) {
          return result(errorEnvelope(error), true);
        }
      };
      handlers.set(name, { schema, run });
      server.registerTool(
        name,
        {
          description: toolDescription(operation, write),
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
