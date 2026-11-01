// Standalone example API rules; source code is parsed and never executed.
import ts from 'typescript';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { collect } from './frontend-ast.mjs';
import {
  computeEntityId,
  computeRelationId,
  computeEvidenceId,
  ENTITY_COMPONENTS,
} from './identity.mjs';

export function collectApi(input, rules) {
  const output = collect(input, { entrypoints: [], httpMethods: [] });
  output.coverage = [];
  output.gaps.forEach((g) => {
    g.capability = 'api';
  });
  const project = input.configSubset.projectId;
  const files = new Map(),
    evidence = new Map(output.evidence.map((e) => [e.id, e]));
  const literal = (node) => (node && ts.isStringLiteralLike(node) ? node.text : undefined);
  const slot = (entity, key) => entity.identity.find((s) => s.name === key)?.value;
  for (const source of input.sources) {
    if (!/\.[cm]?[jt]sx?$/.test(source.path)) continue;
    const ast = ts.createSourceFile(source.path, source.content, ts.ScriptTarget.Latest, true);
    const scopes = input.configSubset.scopes.filter((s) =>
      s.repositoryIds.includes(source.repositoryId),
    );
    files.set(source.repositoryId + ':' + source.path, {
      source,
      ast,
      scope: scopes.length === 1 ? scopes[0].id : 'repo.' + source.repositoryId,
      imports: new Map(),
      exports: new Map(),
      locals: new Map(),
    });
  }
  function cite(file, node) {
    const start = node.getStart(file.ast),
      end = node.getEnd();
    const startByte = Buffer.byteLength(file.source.content.slice(0, start)),
      endByte = Buffer.byteLength(file.source.content.slice(0, end));
    const id = computeEvidenceId(
      file.source.id,
      file.source.contentHash,
      startByte,
      endByte,
      'binding',
    );
    evidence.set(id, {
      id,
      sourceId: file.source.id,
      contentHash: file.source.contentHash,
      startByte,
      endByte,
      startLine: 1 + (file.source.content.slice(0, start).match(/\n/g)?.length ?? 0),
      endLine: 1 + (file.source.content.slice(0, end - 1).match(/\n/g)?.length ?? 0),
      role: 'binding',
    });
    return id;
  }
  function gap(file, node, code, reason) {
    const ref = cite(file, node),
      id =
        'gap_' +
        createHash('sha256')
          .update(JSON.stringify([code, ref]))
          .digest('hex');
    output.gaps.push({
      id,
      code,
      capability: 'api',
      scopeId: file.scope,
      sourceRefs: [file.source.id],
      candidateIds: [],
      reason,
      requiredEvidence: [
        'literal registration and unambiguous imported/local handler and service scope',
      ],
    });
  }
  function entity(file, node, kind, fields, name, attributes = {}) {
    const identity = ENTITY_COMPONENTS[kind].map(([name]) => ({ name, value: fields[name] }));
    const entityId = computeEntityId(project, kind, identity);
    const existing = output.entities.find((e) => e.entityId === entityId);
    if (!existing)
      output.entities.push({
        entityId,
        kind,
        scopeId: file.scope,
        identity,
        assertion: {
          producerKey: input.pluginId,
          name,
          attributes,
          basis: 'observed',
          origin: 'static',
          evidenceRefs: [cite(file, node)],
        },
      });
    return entityId;
  }
  function relation(file, node, kind, fromId, toId, extra = []) {
    const qualifiers = {},
      relationId = computeRelationId(project, kind, fromId, toId, qualifiers);
    if (!output.relations.some((r) => r.relationId === relationId))
      output.relations.push({
        relationId,
        kind,
        fromId,
        toId,
        qualifiers,
        resolution: 'confirmed',
        assertion: {
          producerKey: input.pluginId,
          attributes: {},
          basis: 'observed',
          origin: 'static',
          evidenceRefs: [cite(file, node), ...extra],
        },
      });
  }
  for (const file of files.values()) {
    if (file.ast.parseDiagnostics.length) continue;
    for (const statement of file.ast.statements) {
      if (ts.isImportDeclaration(statement) && literal(statement.moduleSpecifier)) {
        const clause = statement.importClause;
        if (clause?.name)
          file.imports.set(clause.name.text, {
            specifier: literal(statement.moduleSpecifier),
            name: 'default',
          });
        if (clause?.namedBindings && ts.isNamedImports(clause.namedBindings))
          for (const item of clause.namedBindings.elements)
            file.imports.set(item.name.text, {
              specifier: literal(statement.moduleSpecifier),
              name: item.propertyName?.text ?? item.name.text,
            });
      }
      const declarations = ts.isFunctionDeclaration(statement)
        ? [statement]
        : ts.isVariableStatement(statement)
          ? statement.declarationList.declarations
          : [];
      for (const declaration of declarations) {
        const name = declaration.name?.text;
        const operation = output.entities.find(
          (e) =>
            e.kind === 'operation' &&
            slot(e, 'path') === file.source.path &&
            slot(e, 'repositoryId') === file.source.repositoryId &&
            slot(e, 'lexicalPath') === name,
        );
        if (!operation) continue;
        const target = { id: operation.entityId, file, node: declaration };
        if (file.locals.has(name)) file.locals.set(name, null);
        else file.locals.set(name, target);
        if (statement.modifiers?.some((m) => m.kind === ts.SyntaxKind.ExportKeyword))
          file.exports.set(name, target);
      }
    }
  }
  function resolve(file, node) {
    if (!node || !ts.isIdentifier(node)) return undefined;
    if (file.locals.has(node.text)) return file.locals.get(node.text);
    const imported = file.imports.get(node.text);
    if (!imported || !imported.specifier.startsWith('.')) return undefined;
    const relative = path.posix.normalize(
      path.posix.join(path.posix.dirname(file.source.path), imported.specifier),
    );
    const matches = [
      relative,
      ...['.ts', '.tsx', '.js', '.jsx', '/index.ts', '/index.js'].map((ext) => relative + ext),
    ]
      .map((p) => files.get(file.source.repositoryId + ':' + p))
      .filter((f) => f && !f.ast.parseDiagnostics.length);
    return matches.length === 1 ? matches[0].exports.get(imported.name) : undefined;
  }
  let registrations = 0;
  for (const file of files.values()) {
    if (file.ast.parseDiagnostics.length) continue;
    const walk = (node) => {
      if (ts.isCallExpression(node) && ts.isPropertyAccessExpression(node.expression)) {
        const object = node.expression.expression.getText(file.ast),
          method = node.expression.name.text;
        const http = object === rules.httpObject && rules.httpMethods.includes(method);
        const graphql = object === rules.graphqlObject && method === rules.graphqlRegister;
        if (http || graphql) {
          registrations++;
          let nested = false;
          for (let parent = node.parent; parent; parent = parent.parent)
            if (ts.isFunctionLike(parent)) nested = true;
          const services = input.configSubset.services.filter((s) => s.scopeId === file.scope);
          if (nested || services.length !== 1)
            gap(
              file,
              node,
              'AMBIGUOUS_BINDING',
              'Registration ownership/service scope is not proven.',
            );
          else {
            const service = services[0];
            const serviceId = entity(
              file,
              node,
              'service',
              { scopeId: file.scope, configuredId: service.id },
              service.id,
              { protocol: service.protocol },
            );
            const pathOrType = literal(node.arguments[0]);
            const field = graphql ? literal(node.arguments[1]) : undefined;
            if (!pathOrType || (graphql && !field))
              gap(file, node, 'DYNAMIC_TARGET', 'API path or GraphQL schema field is dynamic.');
            else {
              const target = graphql
                ? entity(
                    file,
                    node,
                    'graphqlField',
                    { serviceId: service.id, schemaType: pathOrType, fieldName: field },
                    field,
                    { schemaType: pathOrType, fieldName: field },
                  )
                : entity(
                    file,
                    node,
                    'httpEndpoint',
                    {
                      serviceId: service.id,
                      method: method.toUpperCase(),
                      registeredPath: pathOrType,
                    },
                    pathOrType,
                    {
                      serviceId: service.id,
                      method: method.toUpperCase(),
                      registeredPath: pathOrType,
                    },
                  );
              if (http) relation(file, node, 'serves', serviceId, target);
              const handler = resolve(file, node.arguments[graphql ? 2 : 1]);
              if (handler)
                relation(file, node, graphql ? 'resolves' : 'handles', target, handler.id, [
                  cite(handler.file, handler.node),
                ]);
              else
                gap(
                  file,
                  node,
                  'AMBIGUOUS_BINDING',
                  'Registered handler has no unambiguous lexical/import binding.',
                );
            }
          }
        }
      }
      ts.forEachChild(node, walk);
    };
    walk(file.ast);
  }
  output.evidence = [...evidence.values()];
  for (const scope of new Set([...files.values()].map((f) => f.scope))) {
    const gaps = output.gaps.filter((g) => g.scopeId === scope);
    output.coverage.push({
      capability: 'api',
      scopeId: scope,
      sourceSetId: input.sourceSets[0]?.selectorId ?? 'backend',
      status: gaps.length ? 'partial' : registrations ? 'complete' : 'notConfigured',
      counts: { entities: output.entities.length, relations: output.relations.length },
      gapIds: gaps.map((g) => g.id),
    });
  }
  return output;
}
