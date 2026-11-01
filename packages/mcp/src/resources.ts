import {
  ResourceTemplate,
  ProtocolError,
  INVALID_PARAMS,
  type McpServer,
} from '@modelcontextprotocol/server';
import type { Capabilities, QueryRequest } from '@project-knowledge/sdk';
import { Session, errorEnvelope } from './tools.js';

export function registerResources(server: McpServer, session: Session, capabilities: Capabilities) {
  const content = (uri: URL, value: unknown) => ({
    contents: [{ uri: uri.href, mimeType: 'application/json', text: JSON.stringify(value) }],
  });
  server.registerResource(
    'schema',
    'project-knowledge://schema',
    {
      mimeType: 'application/json',
      description: 'Core capabilities and complete bundled JSON Schema',
    },
    (uri) => content(uri, capabilities),
  );
  for (const name of ['index', 'skill-manifest'])
    server.registerResource(
      name,
      `project-knowledge://${name}`,
      {
        mimeType: 'application/json',
        description:
          name === 'index'
            ? 'Read-only current index status and coverage'
            : 'Core-verified topic/export manifests and freshness',
      },
      async (uri, ctx) => {
        try {
          return content(
            uri,
            await session.execute({ operation: 'status', checkFreshness: true }, false, ctx),
          );
        } catch (error) {
          return content(uri, errorEnvelope(error));
        }
      },
    );
  // Token = base64url(JSON EntityRef or SourceReference); Core checks snapshot ownership.
  for (const name of ['facts', 'evidence'])
    server.registerResource(
      name,
      new ResourceTemplate(`project-knowledge://${name}/{reference}`, { list: undefined }),
      {
        mimeType: 'application/json',
        description: 'Snapshot-bound reference as base64url JSON; freshness checked by Core',
      },
      async (uri, variables, ctx) => {
        const token = variables['reference'];
        if (typeof token !== 'string' || token.length > 16000 || !/^[A-Za-z0-9_-]+$/.test(token))
          throw new ProtocolError(INVALID_PARAMS, 'Invalid reference token');
        let reference: unknown;
        try {
          reference = JSON.parse(Buffer.from(token, 'base64url').toString('utf8'));
        } catch {
          throw new ProtocolError(INVALID_PARAMS, 'Invalid reference token');
        }
        const request =
          name === 'facts'
            ? { operation: 'trace', roots: [reference], direction: 'both', maxDepth: 1 }
            : { operation: 'readSource', references: [reference] };
        try {
          return content(uri, await session.execute(request as QueryRequest, false, ctx));
        } catch (error) {
          return content(uri, errorEnvelope(error));
        }
      },
    );
}
