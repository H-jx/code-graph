// AST mechanics shared by the standalone examples. Registration conventions are
// supplied by each artifact; this module is never loaded by the SDK or Core.
import ts from 'typescript';
import { parse as parseGraphql } from 'graphql';
import path from 'node:path';
import { createHash } from 'node:crypto';
import {
  computeEntityId,
  computeRelationId,
  computeEvidenceId,
  ENTITY_COMPONENTS,
} from './identity.mjs';

const hash = (value) => createHash('sha256').update(JSON.stringify(value)).digest('hex');
const walk = (node, visit) => {
  visit(node);
  ts.forEachChild(node, (child) => walk(child, visit));
};
const literal = (node) =>
  node && (ts.isStringLiteralLike(node) || ts.isNoSubstitutionTemplateLiteral(node))
    ? node.text
    : undefined;
const property = (node, key) =>
  ts.isObjectLiteralExpression(node ?? {})
    ? node.properties.find(
        (p) => ts.isPropertyAssignment(p) && (p.name.text ?? p.name.getText()) === key,
      )?.initializer
    : undefined;
const exported = (node) => node.modifiers?.some((m) => m.kind === ts.SyntaxKind.ExportKeyword);

export function collect(input, rules) {
  const project = input.configSubset.projectId;
  const out = {
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
  };
  const entities = new Map(),
    relations = new Map(),
    evidence = new Map();
  const files = new Map(),
    nodeFunctions = new Map();
  const registrations = new Map();
  const scopes = (source) => {
    const matches = (input.configSubset.scopes ?? []).filter((s) =>
      s.repositoryIds.includes(source.repositoryId),
    );
    return matches.length === 1 ? matches[0].id : `repo.${source.repositoryId}`;
  };
  const local = (file, extra) => ({
    repositoryId: file.source.repositoryId,
    path: file.source.path,
    scopeId: file.scope,
    ...extra,
  });
  function cite(file, node, role = 'binding') {
    const start = typeof node.start === 'number' ? node.start : node.getStart(file.ast);
    const end = typeof node.end === 'number' ? node.end : node.getEnd();
    const startByte = Buffer.byteLength(file.source.content.slice(0, start));
    const endByte = Buffer.byteLength(file.source.content.slice(0, end));
    const id = computeEvidenceId(file.source.id, file.source.contentHash, startByte, endByte, role);
    evidence.set(id, {
      id,
      sourceId: file.source.id,
      contentHash: file.source.contentHash,
      startByte,
      endByte,
      startLine: 1 + (file.source.content.slice(0, start).match(/\n/g)?.length ?? 0),
      endLine:
        1 + (file.source.content.slice(0, Math.max(start, end - 1)).match(/\n/g)?.length ?? 0),
      role,
    });
    return id;
  }
  function gap(file, node, code, capability, reason, requiredEvidence, candidateIds = []) {
    const ref = cite(file, node);
    const id = `gap_${hash([code, ref, capability])}`;
    if (!out.gaps.some((g) => g.id === id))
      out.gaps.push({
        id,
        code,
        capability,
        scopeId: file.scope,
        sourceRefs: [file.source.id],
        candidateIds,
        reason,
        requiredEvidence,
      });
  }
  function entity(file, node, kind, fields, name, attributes = {}) {
    const identity = ENTITY_COMPONENTS[kind].map(([key]) => ({ name: key, value: fields[key] }));
    const entityId = computeEntityId(project, kind, identity);
    const ref = cite(file, node);
    if (entities.has(entityId)) {
      const refs = entities.get(entityId).assertion.evidenceRefs;
      if (!refs.includes(ref)) refs.push(ref);
      return entityId;
    }
    entities.set(entityId, {
      entityId,
      kind,
      scopeId:
        fields.scopeId ??
        (fields.serviceId &&
          input.configSubset.services?.find((s) => s.id === fields.serviceId)?.scopeId) ??
        file.scope,
      identity,
      assertion: {
        producerKey: input.pluginId,
        name,
        attributes,
        basis: 'observed',
        origin: 'static',
        evidenceRefs: [ref],
      },
    });
    return entityId;
  }
  function relate(file, node, kind, fromId, toId, refs = []) {
    const qualifiers = {};
    const relationId = computeRelationId(project, kind, fromId, toId, qualifiers);
    relations.set(relationId, {
      relationId,
      kind,
      fromId,
      toId,
      qualifiers,
      resolution: 'confirmed',
      assertion: {
        producerKey: input.pluginId,
        basis: 'observed',
        origin: 'static',
        attributes: {},
        evidenceRefs: [cite(file, node), ...refs],
      },
    });
  }
  for (const source of input.sources) {
    if (!/\.[cm]?[jt]sx?$/.test(source.path)) continue;
    const ast = ts.createSourceFile(
      source.path,
      source.content,
      ts.ScriptTarget.Latest,
      true,
      /\.tsx$/.test(source.path)
        ? ts.ScriptKind.TSX
        : /\.jsx$/.test(source.path)
          ? ts.ScriptKind.JSX
          : ts.ScriptKind.TS,
    );
    const file = {
      source,
      ast,
      scope: scopes(source),
      imports: new Map(),
      exports: new Map(),
      variables: new Map(),
      declarations: [],
    };
    files.set(source.repositoryId + ':' + source.path, file);
    if (ast.parseDiagnostics.length) {
      gap(
        file,
        ast,
        'UNSUPPORTED_SYNTAX',
        'frontend',
        'Source has parse errors; no declarations from this file are confirmed.',
        ['valid syntax'],
      );
      file.invalid = true;
    }
  }
  function resolveFile(file, specifier) {
    if (!specifier.startsWith('.')) return undefined;
    const base = path.posix.normalize(
      path.posix.join(path.posix.dirname(file.source.path), specifier),
    );
    const matches = [
      base,
      ...['.ts', '.tsx', '.js', '.jsx', '/index.ts', '/index.tsx', '/index.js'].map(
        (ext) => base + ext,
      ),
    ]
      .map((p) => files.get(file.source.repositoryId + ':' + p))
      .filter((f) => f && !f.invalid);
    return matches.length === 1 ? matches[0] : undefined;
  }
  function enclosing(node) {
    for (let n = node.parent; n; n = n.parent)
      if (nodeFunctions.has(n)) return nodeFunctions.get(n);
  }
  // Bind functions lexically before resolving any calls or imports.
  for (const file of files.values()) {
    if (file.invalid) continue;
    for (const statement of file.ast.statements) {
      if (ts.isImportDeclaration(statement) && literal(statement.moduleSpecifier)) {
        const clause = statement.importClause;
        if (clause?.name)
          file.imports.set(clause.name.text, {
            file: literal(statement.moduleSpecifier),
            export: 'default',
          });
        if (clause?.namedBindings && ts.isNamedImports(clause.namedBindings))
          for (const item of clause.namedBindings.elements)
            file.imports.set(item.name.text, {
              file: literal(statement.moduleSpecifier),
              export: item.propertyName?.text ?? item.name.text,
            });
      }
      if (ts.isVariableStatement(statement))
        for (const decl of statement.declarationList.declarations)
          if (ts.isIdentifier(decl.name)) file.variables.set(decl.name.text, decl.initializer);
    }
    walk(file.ast, (node) => {
      const isFunction =
        ts.isFunctionDeclaration(node) ||
        ((ts.isArrowFunction(node) || ts.isFunctionExpression(node)) &&
          ts.isVariableDeclaration(node.parent));
      if (!isFunction || !node.body) return;
      const name = node.name?.text ?? node.parent.name?.text;
      if (!name) return;
      const parent = enclosing(node);
      const lexicalPath = parent ? parent.lexicalPath + '.' + name : name;
      // JSX must be in this function, not merely in a nested function.
      let jsx = false;
      const ownWalk = (n) => {
        if (n !== node && ts.isFunctionLike(n)) return;
        if (ts.isJsxElement(n) || ts.isJsxSelfClosingElement(n) || ts.isJsxFragment(n)) jsx = true;
        ts.forEachChild(n, ownWalk);
      };
      ownWalk(node);
      const kind = jsx ? 'component' : 'operation';
      const fields = local(file, {
        lexicalPath,
        ...(jsx
          ? {}
          : {
              normalizedSignature: (() => {
                const tokens = (part) => {
                  if (
                    part.kind >= ts.SyntaxKind.FirstJSDocNode &&
                    part.kind <= ts.SyntaxKind.LastJSDocNode
                  )
                    return '';
                  const children = part.getChildren(file.ast);
                  return children.length ? children.map(tokens).join('') : part.getText(file.ast);
                };
                return '(' + node.parameters.map(tokens).join(',') + ')';
              })(),
            }),
      });
      const id = entity(file, node, kind, fields, name);
      const declaration = { id, name, lexicalPath, parent, file, node, kind };
      nodeFunctions.set(node, declaration);
      file.declarations.push(declaration);
      const declarationStatement = ts.isVariableDeclaration(node.parent)
        ? node.parent.parent.parent
        : node;
      if (!parent && exported(declarationStatement)) file.exports.set(name, declaration);
      if (
        !parent &&
        declarationStatement.modifiers?.some((m) => m.kind === ts.SyntaxKind.DefaultKeyword)
      )
        file.exports.set('default', declaration);
      if (parent) relate(file, node, 'contains', parent.id, id);
      const comments =
        ts.getLeadingCommentRanges(file.source.content, declarationStatement.pos) ?? [];
      for (const comment of comments.filter((c) =>
        file.source.content.slice(c.pos, c.end).startsWith('/**'),
      )) {
        const text = file.source.content
          .slice(comment.pos, comment.end)
          .replace(/^\/\*\*|\*\/$/g, '')
          .replace(/^\s*\* ?/gm, '')
          .trim();
        const commentId = entity(
          file,
          { start: comment.pos, end: comment.end },
          'comment',
          local(file, { lexicalPath: lexicalPath + '.doc' }),
          text,
          { text },
        );
        relate(file, { start: comment.pos, end: comment.end }, 'describes', commentId, id);
      }
    });
    for (const statement of file.ast.statements)
      if (ts.isExportAssignment(statement) && ts.isIdentifier(statement.expression)) {
        const target = file.declarations.find(
          (d) => !d.parent && d.name === statement.expression.text,
        );
        if (target) file.exports.set('default', target);
      }
  }
  function resolveName(file, name, owner) {
    for (let current = owner; current; current = current.parent) {
      if (current.node.parameters.some((p) => p.name.getText(file.ast) === name)) return undefined;
      const localDeclaration = file.declarations.filter(
        (d) => d.parent === current && d.name === name,
      );
      if (localDeclaration.length === 1) return localDeclaration[0];
      if (localDeclaration.length > 1) return undefined;
    }
    if (shadowsImport(name, owner)) return undefined;
    const top = file.declarations.filter((d) => !d.parent && d.name === name);
    if (top.length === 1) return top[0];
    const imported = file.imports.get(name);
    return imported ? resolveFile(file, imported.file)?.exports.get(imported.export) : undefined;
  }
  function shadowsImport(name, owner) {
    const binds = (node) =>
      ts.isIdentifier(node)
        ? node.text === name
        : node.elements?.some((e) => e.name && binds(e.name));
    for (let current = owner; current; current = current.parent) {
      if (current.node.parameters.some((p) => binds(p.name))) return true;
      let shadow = false;
      const inspect = (node) => {
        if (ts.isFunctionLike(node)) return;
        if (
          ts.isVariableDeclaration(node) &&
          binds(node.name) &&
          !(node.initializer && ts.isFunctionLike(node.initializer))
        )
          shadow = true;
        ts.forEachChild(node, inspect);
      };
      inspect(current.node.body);
      if (shadow) return true;
    }
    return false;
  }
  function resolveClient(file, name, owner) {
    if (shadowsImport(name, owner)) return undefined;
    const imported = file.imports.get(name);
    const provider = imported ? resolveFile(file, imported.file) : file;
    const initializer = provider?.variables.get(imported?.export ?? name);
    if (
      !initializer ||
      !ts.isCallExpression(initializer) ||
      initializer.expression.getText(provider.ast) !== rules.clientFactory
    )
      return undefined;
    const options = initializer.arguments[0];
    const serviceId = literal(property(options, 'serviceId'));
    const baseURL = literal(property(options, 'baseURL'));
    const binding = (input.configSubset.bindings ?? []).filter(
      (b) =>
        b.kind === 'clientToService' &&
        b.scopeId === file.scope &&
        b.from === name &&
        b.to === serviceId,
    );
    if (!serviceId || baseURL === undefined || binding.length !== 1) return undefined;
    const proxyNode = property(options, 'proxy');
    let proxy;
    if (proxyNode && ts.isObjectLiteralExpression(proxyNode)) {
      const entries = proxyNode.properties.map((p) =>
        ts.isPropertyAssignment(p) && literal(p.initializer) !== undefined
          ? [p.name.text ?? p.name.getText(provider.ast), literal(p.initializer)]
          : undefined,
      );
      if (entries.every(Boolean)) proxy = Object.fromEntries(entries);
    }
    return {
      serviceId,
      baseURL,
      ...(proxy ? { proxy } : {}),
      bindingComplete: !proxyNode || !!proxy,
      reference: cite(provider, initializer),
    };
  }
  for (const file of files.values()) {
    if (file.invalid) continue;
    walk(file.ast, (node) => {
      const owner = enclosing(node);
      if (ts.isCallExpression(node) && owner) {
        if (ts.isIdentifier(node.expression)) {
          const target = resolveName(file, node.expression.text, owner);
          if (target)
            relate(file, node, 'calls', owner.id, target.id, [cite(target.file, target.node)]);
        }
        if (
          ts.isPropertyAccessExpression(node.expression) &&
          ts.isIdentifier(node.expression.expression)
        ) {
          const clientName = node.expression.expression.text;
          const method = node.expression.name.text;
          if (!rules.clientFactory || ![...rules.httpMethods, 'graphql'].includes(method)) return;
          const client = resolveClient(file, clientName, owner);
          const requestKey = file.source.path + '#' + owner.lexicalPath;
          const target = literal(node.arguments[0]);
          const attributes = {
            expression: node.arguments[0]?.getText(file.ast) ?? '',
            clientName,
            ...(client
              ? {
                  serviceId: client.serviceId,
                  baseURL: client.baseURL,
                  bindingComplete: client.bindingComplete,
                  ...(client.proxy ? { proxy: client.proxy } : {}),
                }
              : { bindingComplete: false }),
          };
          if (method !== 'graphql' && target !== undefined)
            Object.assign(attributes, { method: method.toUpperCase(), path: target });
          const request = entity(
            file,
            node,
            'request',
            { scopeId: file.scope, requestKey },
            owner.name,
            attributes,
          );
          relate(file, node, 'requests', owner.id, request, client ? [client.reference] : []);
          if (!client || !client.bindingComplete || target === undefined) {
            gap(
              file,
              node,
              'DYNAMIC_TARGET',
              'frontend',
              'Request target or client service binding is not statically proven.',
              ['literal target and explicit client/service binding'],
            );
          } else if (method === 'graphql') {
            try {
              const document = parseGraphql(target);
              for (const definition of document.definitions) {
                if (definition.kind !== 'OperationDefinition' || !definition.name) {
                  gap(
                    file,
                    node,
                    'DYNAMIC_TARGET',
                    'frontend',
                    'Only named GraphQL operations are supported.',
                    ['named operation'],
                  );
                  continue;
                }
                const operation = entity(
                  file,
                  node,
                  'graphqlOperation',
                  {
                    serviceId: client.serviceId,
                    operationType: definition.operation,
                    operationName: definition.name.value,
                  },
                  definition.name.value,
                  {
                    operationType: definition.operation,
                    operationName: definition.name.value,
                    variables: definition.variableDefinitions.map((v) => v.variable.name.value),
                  },
                );
                relate(file, node, 'requests', owner.id, operation, [client.reference]);
                relate(file, node, 'references', request, operation, [client.reference]);
                const schemaType =
                  definition.operation[0].toUpperCase() + definition.operation.slice(1);
                for (const selection of definition.selectionSet.selections) {
                  if (selection.kind !== 'Field') {
                    gap(
                      file,
                      node,
                      'DYNAMIC_TARGET',
                      'frontend',
                      'GraphQL fragment target needs schema/fragment analysis.',
                      ['field selection'],
                    );
                    continue;
                  }
                  const field = entity(
                    file,
                    node,
                    'graphqlField',
                    { serviceId: client.serviceId, schemaType, fieldName: selection.name.value },
                    selection.name.value,
                    { schemaType, fieldName: selection.name.value },
                  );
                  // The full literal/call is the proven source span; escaped text is not assigned invented token offsets.
                  relate(file, node, 'references', operation, field);
                }
              }
            } catch {
              gap(
                file,
                node,
                'UNSUPPORTED_SYNTAX',
                'frontend',
                'GraphQL document failed parsing.',
                ['valid GraphQL document'],
              );
            }
          }
        }
      }
      if (ts.isJsxAttribute(node) && node.name.text.startsWith('on') && owner) {
        const expression =
          node.initializer && ts.isJsxExpression(node.initializer)
            ? node.initializer.expression
            : undefined;
        const handler =
          expression && ts.isIdentifier(expression)
            ? resolveName(file, expression.text, owner)
            : undefined;
        if (!handler) {
          gap(
            file,
            node,
            'DYNAMIC_TARGET',
            'frontend',
            'JSX event handler is not a direct lexical binding.',
            ['named handler binding'],
          );
          return;
        }
        const lexicalPath = owner.lexicalPath + '.' + node.name.text + '.' + expression.text;
        const event = entity(file, node, 'event', local(file, { lexicalPath }), node.name.text, {
          event: node.name.text,
        });
        relate(file, node, 'triggers', event, handler.id, [cite(handler.file, handler.node)]);
      }
    });
    const moduleInfo = file.variables.get(rules.moduleExport);
    const moduleKey = literal(property(moduleInfo, 'key'));
    if (moduleKey)
      entity(
        file,
        moduleInfo,
        'module',
        { scopeId: file.scope, semanticKey: moduleKey },
        literal(property(moduleInfo, 'name')) ?? moduleKey,
        { name: literal(property(moduleInfo, 'name')) ?? moduleKey },
      );
    walk(file.ast, (node) => {
      if (!ts.isCallExpression(node)) return;
      const registration = rules.entrypoints.find(
        (r) => node.expression.getText(file.ast) === r.call,
      );
      if (!registration) return;
      if (enclosing(node)) {
        gap(
          file,
          node,
          'DYNAMIC_TARGET',
          'entrypoints',
          'Nested registration requires runtime ownership evidence.',
          ['top-level registration or proven runtime instance'],
        );
        return;
      }
      const semanticKey = literal(node.arguments[0]);
      const handler =
        node.arguments[1] && ts.isIdentifier(node.arguments[1])
          ? resolveName(file, node.arguments[1].text, enclosing(node))
          : undefined;
      if (!semanticKey || !handler) {
        gap(
          file,
          node,
          'DYNAMIC_TARGET',
          'entrypoints',
          'Entrypoint registration has no proven handler.',
          ['literal key and imported/local handler'],
        );
        return;
      }
      const entry = entity(
        file,
        node,
        'entrypoint',
        { scopeId: file.scope, semanticKey },
        semanticKey,
        { entryKind: registration.kind },
      );
      relate(file, node, 'handles', entry, handler.id, [cite(handler.file, handler.node)]);
    });
    const routeArray = file.variables.get(rules.routesExport);
    if (!routeArray) continue;
    registrations.set(file.scope, (registrations.get(file.scope) ?? 0) + 1);
    const router = file.variables.get(rules.routerExport);
    const routerId = literal(property(router, 'id')),
      mode = literal(property(router, 'mode'));
    if (
      !routerId ||
      !['hash', 'history'].includes(mode) ||
      !ts.isArrayLiteralExpression(routeArray)
    ) {
      gap(
        file,
        routeArray,
        'DYNAMIC_TARGET',
        'routes',
        'Router identity/mode or route list is dynamic.',
        ['literal router identity, mode and route list'],
      );
      continue;
    }
    for (const route of routeArray.elements) {
      const declaredPath = literal(property(route, rules.pathKey));
      if (!declaredPath) {
        gap(file, route, 'DYNAMIC_TARGET', 'routes', 'Route path is dynamic.', [
          'literal route path',
        ]);
        continue;
      }
      const aliasNode = property(route, 'alias');
      let alias = literal(aliasNode);
      if (
        aliasNode &&
        ts.isArrayLiteralExpression(aliasNode) &&
        aliasNode.elements.every((n) => literal(n) !== undefined)
      )
        alias = aliasNode.elements.map(literal);
      if (aliasNode && alias === undefined)
        gap(file, aliasNode, 'DYNAMIC_TARGET', 'routes', 'Route alias is dynamic.', [
          'literal alias or alias array',
        ]);
      const attributes = {
        declaredPath,
        routerId,
        mode,
        ...(alias !== undefined ? { alias } : {}),
      };
      const routeId = entity(
        file,
        route,
        'route',
        { scopeId: file.scope, routerId, mode, declaredPath },
        declaredPath,
        attributes,
      );
      const componentNode = property(route, rules.componentKey);
      let component;
      if (componentNode && ts.isIdentifier(componentNode))
        component = resolveName(file, componentNode.text);
      else if (
        componentNode &&
        ts.isArrowFunction(componentNode) &&
        ts.isCallExpression(componentNode.body) &&
        componentNode.body.expression.kind === ts.SyntaxKind.ImportKeyword &&
        literal(componentNode.body.arguments[0])
      )
        component = resolveFile(file, literal(componentNode.body.arguments[0]))?.exports.get(
          'default',
        );
      if (component?.kind === 'component')
        relate(file, route, 'renders', routeId, component.id, [
          cite(component.file, component.node),
        ]);
      else
        gap(file, route, 'DYNAMIC_TARGET', 'routes', 'Route component binding cannot be proven.', [
          'direct component import or literal lazy import',
        ]);
      const key = literal(property(route, rules.titleKey));
      if (key) {
        const matches = [];
        for (const messageFile of files.values()) {
          if (messageFile.invalid || messageFile.scope !== file.scope) continue;
          const messages = messageFile.variables.get(rules.messagesExport);
          const title = property(messages, key);
          if (literal(title) !== undefined)
            matches.push({ file: messageFile, node: title, text: literal(title) });
        }
        if (matches.length === 1) {
          const message = matches[0];
          const title = entity(
            message.file,
            message.node,
            'title',
            local(message.file, { lexicalPath: key }),
            message.text,
            { text: message.text, i18nKey: key },
          );
          relate(file, route, 'describes', title, routeId, [cite(message.file, message.node)]);
        } else
          gap(
            file,
            route,
            'AMBIGUOUS_BINDING',
            'routes',
            'Title key has zero or multiple literal definitions.',
            ['unique i18n binding'],
          );
      } else if (property(route, rules.titleKey)) {
        gap(file, route, 'DYNAMIC_TARGET', 'routes', 'Title key is dynamic.', ['literal i18n key']);
      } else if (rules.titleLiteralKey && property(route, rules.titleLiteralKey)) {
        const node = property(route, rules.titleLiteralKey);
        const text = literal(node);
        if (text !== undefined) {
          const title = entity(
            file,
            node,
            'title',
            local(file, { lexicalPath: routerId + '.' + declaredPath + '.title' }),
            text,
            { text },
          );
          relate(file, route, 'describes', title, routeId);
        } else
          gap(file, node, 'DYNAMIC_TARGET', 'routes', 'Inline title is dynamic.', [
            'literal title',
          ]);
      }
    }
  }
  if (files.size === 0) {
    for (const scope of (input.configSubset.scopes ?? []).filter((s) => s.kind === 'application'))
      for (const capability of ['routes', 'frontend', 'comments', 'modules', 'entrypoints'])
        out.coverage.push({
          capability,
          scopeId: scope.id,
          sourceSetId: input.sourceSets[0]?.selectorId ?? 'frontend',
          status: 'notConfigured',
          counts: { entities: 0, relations: 0 },
          gapIds: [],
        });
  }
  for (const scope of new Set([...files.values()].map((f) => f.scope)))
    for (const capability of ['routes', 'frontend', 'comments', 'modules', 'entrypoints']) {
      const gaps = out.gaps.filter(
        (g) =>
          g.scopeId === scope && (g.capability === capability || g.code === 'UNSUPPORTED_SYNTAX'),
      );
      const configured =
        capability === 'routes'
          ? registrations.has(scope)
          : capability === 'modules' || capability === 'entrypoints'
            ? [...entities.values()].some(
                (e) =>
                  e.scopeId === scope &&
                  e.kind === (capability === 'modules' ? 'module' : 'entrypoint'),
              )
            : true;
      out.coverage.push({
        capability,
        scopeId: scope,
        sourceSetId: input.sourceSets[0]?.selectorId ?? 'frontend',
        status: gaps.length ? 'partial' : configured ? 'complete' : 'notConfigured',
        counts: { entities: entities.size, relations: relations.size },
        gapIds: gaps.map((g) => g.id),
      });
    }
  out.entities = [...entities.values()];
  out.relations = [...relations.values()];
  out.evidence = [...evidence.values()];
  return out;
}
