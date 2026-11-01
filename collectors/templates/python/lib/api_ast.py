"""Project-example Python registrations, parsed without importing business code."""
import ast
import json
import pathlib

from identity import evaluate, stable_id


def collect_api(payload, rules):
    specs = {kind['kind']: kind['identityComponents'] for kind in json.loads(
        pathlib.Path(__file__).with_name('identity-spec.json').read_text(encoding='utf-8'))['entityKinds']}
    project = payload['configSubset']['projectId']
    output = dict(protocolVersion='1.0', runId=payload['runId'], stageId=payload['stageId'],
                  inputSnapshotId=payload['inputSnapshotId'], entities=[], relations=[], evidence=[], dependencies=[], coverage=[], gaps=[])
    evidence = {}
    files = {}

    def cite(file, node):
        source = file['source']
        lines = source['content'].encode('utf-8').splitlines(keepends=True)
        start = sum(map(len, lines[:node.lineno - 1])) + node.col_offset
        end = sum(map(len, lines[:node.end_lineno - 1])) + node.end_col_offset
        identifier = stable_id('ev_', [source['id'], source['contentHash'], start, end, 'binding'])
        evidence[identifier] = dict(id=identifier, sourceId=source['id'], contentHash=source['contentHash'],
                                    startByte=start, endByte=end, startLine=node.lineno, endLine=node.end_lineno, role='binding')
        return identifier

    def gap(file, node, code, reason):
        ref = cite(file, node)
        identifier = stable_id('gap_', [code, ref])
        output['gaps'].append(dict(id=identifier, code=code, capability='api', scopeId=file['scope'],
                                   sourceRefs=[file['source']['id']], candidateIds=[], reason=reason,
                                   requiredEvidence=['literal registration, explicit service scope and lexical/imported handler']))

    def entity(file, node, kind, fields, name, attributes=None):
        slots = [dict(name=spec['name'], **({'value': fields[spec['name']]} if spec['name'] in fields else {})) for spec in specs[kind]]
        identifier = evaluate(dict(type='entity', projectId=project, kind=kind, slots=slots), specs)
        if not any(item['entityId'] == identifier for item in output['entities']):
            output['entities'].append(dict(entityId=identifier, kind=kind, scopeId=file['scope'], identity=slots,
                assertion=dict(producerKey=payload['pluginId'], name=name, attributes=attributes or {}, basis='observed', origin='static', evidenceRefs=[cite(file, node)])))
        return identifier

    def relation(file, node, kind, source, target, refs=()):
        identifier = stable_id('rel_', [project, kind, source, target, {}])
        if not any(item['relationId'] == identifier for item in output['relations']):
            output['relations'].append(dict(relationId=identifier, kind=kind, fromId=source, toId=target, qualifiers={}, resolution='confirmed',
                assertion=dict(producerKey=payload['pluginId'], attributes={}, basis='observed', origin='static', evidenceRefs=[cite(file, node), *refs])))

    def literal(node):
        return node.value if isinstance(node, ast.Constant) and isinstance(node.value, str) else None

    for source in payload['sources']:
        if not source['path'].endswith('.py'):
            continue
        scopes = [scope for scope in payload['configSubset']['scopes'] if source['repositoryId'] in scope['repositoryIds']]
        file = dict(source=source, scope=scopes[0]['id'] if len(scopes) == 1 else 'repo.' + source['repositoryId'], functions={}, imports={})
        files[(source['repositoryId'], source['path'])] = file
        try:
            file['tree'] = ast.parse(source['content'], filename=source['path'])
        except SyntaxError:
            # A whole nonempty source span is honest when no AST node exists.
            node = ast.Constant(value='invalid')
            node.lineno = node.end_lineno = 1
            node.col_offset = 0
            node.end_col_offset = len(source['content'].splitlines()[0].encode('utf-8'))
            if node.end_col_offset:
                gap(file, node, 'UNSUPPORTED_SYNTAX', 'Python source failed parsing; no facts from this file are confirmed.')
            continue
        for node in file['tree'].body:
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                fields = dict(repositoryId=source['repositoryId'], path=source['path'], scopeId=file['scope'],
                              lexicalPath=node.name, normalizedSignature='(' + ','.join(arg.arg for arg in node.args.args) + ')')
                identifier = entity(file, node, 'operation', fields, node.name)
                # Duplicate bindings are ambiguous, not a first-match decision.
                target = dict(id=identifier, file=file, node=node)
                file['functions'][node.name] = None if node.name in file['functions'] else target
            elif isinstance(node, ast.ImportFrom) and node.level == 1 and node.module:
                for alias in node.names:
                    file['imports'][alias.asname or alias.name] = (node.module, alias.name)

    def resolve(file, node):
        if not isinstance(node, ast.Name):
            return None
        if node.id in file['functions']:
            return file['functions'][node.id]
        imported = file['imports'].get(node.id)
        if not imported:
            return None
        module, name = imported
        target_path = str(pathlib.PurePosixPath(file['source']['path']).parent / (module.replace('.', '/') + '.py'))
        provider = files.get((file['source']['repositoryId'], target_path))
        return provider['functions'].get(name) if provider else None

    registrations = 0
    for file in files.values():
        if 'tree' not in file:
            continue
        for statement in file['tree'].body:
            if not isinstance(statement, ast.Expr) or not isinstance(statement.value, ast.Call):
                continue
            node = statement.value
            http = isinstance(node.func, ast.Call) and isinstance(node.func.func, ast.Attribute) and isinstance(node.func.func.value, ast.Name) and node.func.func.value.id == rules['httpObject'] and node.func.func.attr in rules['httpMethods']
            graphql = isinstance(node.func, ast.Attribute) and isinstance(node.func.value, ast.Name) and node.func.value.id == rules['graphqlObject'] and node.func.attr == rules['graphqlRegister']
            if not http and not graphql:
                continue
            registrations += 1
            services = [service for service in payload['configSubset']['services'] if service['scopeId'] == file['scope']]
            if len(services) != 1:
                gap(file, node, 'AMBIGUOUS_BINDING', 'Service ownership is not uniquely configured for this scope.')
                continue
            service = services[0]
            service_id = entity(file, node, 'service', dict(scopeId=file['scope'], configuredId=service['id']), service['id'], dict(protocol=service['protocol']))
            args = node.func.args if http else node.args
            name = literal(args[0]) if args else None
            field = literal(args[1]) if graphql and len(args) > 1 else None
            if not name or graphql and not field:
                gap(file, node, 'DYNAMIC_TARGET', 'HTTP path or GraphQL schema field is dynamic.')
                continue
            if http:
                target = entity(file, node, 'httpEndpoint', dict(serviceId=service['id'], method=node.func.func.attr.upper(), registeredPath=name), name,
                                dict(serviceId=service['id'], method=node.func.func.attr.upper(), registeredPath=name))
                relation(file, node, 'serves', service_id, target)
                handler = resolve(file, node.args[0] if node.args else None)
            else:
                target = entity(file, node, 'graphqlField', dict(serviceId=service['id'], schemaType=name, fieldName=field), field, dict(schemaType=name, fieldName=field))
                handler = resolve(file, node.args[2] if len(node.args) > 2 else None)
            if handler:
                relation(file, node, 'handles' if http else 'resolves', target, handler['id'], [cite(handler['file'], handler['node'])])
            else:
                gap(file, node, 'AMBIGUOUS_BINDING', 'Registration handler has no proven local/import binding.')
        for function in file['functions'].values():
            if not function:
                continue
            calls = []
            local_bindings = set()
            def inspect(node):
                if node is not function['node'] and isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.Lambda, ast.ClassDef)):
                    return
                if isinstance(node, ast.Name) and isinstance(node.ctx, ast.Store):
                    local_bindings.add(node.id)
                if isinstance(node, ast.Call):
                    calls.append(node)
                for child in ast.iter_child_nodes(node):
                    inspect(child)
            inspect(function['node'])
            for call in calls:
                if isinstance(call, ast.Call) and isinstance(call.func, ast.Name):
                    if call.func.id in local_bindings or any(arg.arg == call.func.id for arg in function['node'].args.args):
                        continue
                    target = resolve(file, call.func)
                    if target:
                        relation(file, call, 'calls', function['id'], target['id'], [cite(target['file'], target['node'])])
    output['evidence'] = list(evidence.values())
    for scope in sorted({file['scope'] for file in files.values()}):
        gaps = [item for item in output['gaps'] if item['scopeId'] == scope]
        output['coverage'].append(dict(capability='api', scopeId=scope, sourceSetId=payload['sourceSets'][0]['selectorId'],
                                       status='partial' if gaps else 'complete' if registrations else 'notConfigured',
                                       counts=dict(entities=len(output['entities']), relations=len(output['relations'])), gapIds=[item['id'] for item in gaps]))
    return output
