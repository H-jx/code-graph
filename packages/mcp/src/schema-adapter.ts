import type { StandardSchemaWithJSON } from '@modelcontextprotocol/server';
import { ProjectKnowledge, ProjectKnowledgeError } from '@project-knowledge/sdk';

type Json = Record<string, unknown>;
/** Project a tagged Core branch without maintaining another validation model. */
export function coreSchema(
  document: unknown,
  operation: string,
  maintenance: boolean,
): StandardSchemaWithJSON<Json> {
  const schema = document as Json;
  const defs = schema['$defs'] as Record<string, Json>;
  const union = maintenance ? defs['MaintenanceRequest'] : schema;
  const branches = (union?.['oneOf'] ?? union?.['anyOf']) as Json[];
  const branch = branches.find(
    (item) => (item['properties'] as Record<string, Json>)?.['operation']?.['const'] === operation,
  );
  if (!branch) throw new Error(`Core schema is missing operation ${operation}`);
  const properties = { ...(branch['properties'] as Json) };
  delete properties['operation'];
  const input = {
    ...branch,
    $schema: schema['$schema'],
    $defs: defs,
    properties,
    required: ((branch['required'] ?? []) as string[]).filter((key) => key !== 'operation'),
  };
  return {
    '~standard': {
      version: 1,
      vendor: 'project-knowledge-core',
      jsonSchema: { input: () => input, output: () => input },
      async validate(value) {
        if (!value || typeof value !== 'object' || Array.isArray(value))
          return { issues: [{ message: 'Expected object' }] };
        if ('operation' in value)
          return {
            issues: [{ message: 'Operation is selected by the tool name', path: ['operation'] }],
          };
        try {
          await ProjectKnowledge.validateRequest({ ...value, operation }, maintenance);
          return { value: value as Json };
        } catch (error) {
          if (!(error instanceof ProjectKnowledgeError))
            return { issues: [{ message: 'Core validation unavailable' }] };
          return {
            issues: error.issues.length
              ? error.issues.map((issue) => ({
                  message: JSON.stringify(issue),
                  path: issue.instancePath
                    .split('/')
                    .slice(1)
                    .map((part) => part.replaceAll('~1', '/').replaceAll('~0', '~')),
                }))
              : [{ message: `${error.code}: ${error.message}` }],
          };
        }
      },
    },
  };
}
