"""Example database collector. Parses snapshots only; never executes SQL or source."""
import json
import pathlib

import sqlglot
from sqlglot import exp
from sqlglot.errors import ErrorLevel, ParseError

from identity import evaluate, stable_id
from source_ast import imported_path, parse_source


class DatabaseCollector:
    def __init__(self, payload, rules):
        self.input, self.rules = payload, rules
        self.project = payload['configSubset']['projectId']
        self.specs = {item['kind']: item['identityComponents'] for item in json.loads(
            pathlib.Path(__file__).with_name('identity-spec.json').read_text(encoding='utf-8'))['entityKinds']}
        self.output = dict(protocolVersion='1.0', runId=payload['runId'], stageId=payload['stageId'],
                           inputSnapshotId=payload['inputSnapshotId'], entities=[], relations=[], evidence=[], dependencies=[], coverage=[], gaps=[])
        self.entities, self.relations, self.evidence = {}, {}, {}
        self.tables, self.files, self.sources = {}, {}, {}
        self.data_sources = {item['id']: item for item in payload['configSubset']['dataSources']}

    def cite(self, source, span=None):
        data = source['content'].encode('utf-8')
        start, end = span or (0, len(data))
        identifier = stable_id('ev_', [source['id'], source['contentHash'], start, end, 'binding'])
        self.evidence[identifier] = dict(id=identifier, sourceId=source['id'], contentHash=source['contentHash'],
            startByte=start, endByte=end, startLine=data[:start].count(b'\n') + 1,
            endLine=data[:max(start, end - 1)].count(b'\n') + 1, role='binding')
        return identifier

    def scope(self, source):
        scopes = [item['id'] for item in self.input['configSubset']['scopes'] if source['repositoryId'] in item['repositoryIds']]
        return scopes[0] if len(scopes) == 1 else 'repo.' + source['repositoryId']

    def gap(self, source, code, reason, span=None):
        ref = self.cite(source, span)
        identifier = stable_id('gap_', [ref, code, reason])
        if not any(item['id'] == identifier for item in self.output['gaps']):
            self.output['gaps'].append(dict(id=identifier, code=code, capability='database', scopeId=self.scope(source),
                sourceRefs=[source['id']], candidateIds=[], reason=reason,
                requiredEvidence=['explicit dataSource/instance binding, supported static AST and complete ordered keys']))
        return identifier

    def entity(self, source, kind, fields, name, attributes=None, span=None, basis='observed', refs=()):
        slots = [dict(name=spec['name'], **({'value': fields[spec['name']]} if spec['name'] in fields else {})) for spec in self.specs[kind]]
        identifier = evaluate(dict(type='entity', projectId=self.project, kind=kind, slots=slots), self.specs)
        citations = [self.cite(source, span), *refs]
        if identifier not in self.entities:
            self.entities[identifier] = dict(entityId=identifier, kind=kind, scopeId=fields.get('scopeId', self.scope(source)), identity=slots,
                assertion=dict(producerKey=self.input['pluginId'], name=name, attributes=attributes or {}, basis=basis, origin='static', evidenceRefs=citations))
        else:
            existing = self.entities[identifier]['assertion']['evidenceRefs']
            existing.extend(ref for ref in citations if ref not in existing)
        return identifier

    def relation(self, source, kind, origin, target, qualifiers=None, span=None, basis='observed', refs=()):
        qualifiers = qualifiers or {}
        identifier = stable_id('rel_', [self.project, kind, origin, target, qualifiers])
        self.relations[identifier] = dict(relationId=identifier, kind=kind, fromId=origin, toId=target,
            qualifiers=qualifiers, resolution='confirmed', assertion=dict(producerKey=self.input['pluginId'], attributes={},
            basis=basis, origin='static', evidenceRefs=[self.cite(source, span), *refs]))

    def data_source(self, source, name, span=None):
        config = self.data_sources.get(name)
        if not config or config['scopeId'] != self.scope(source):
            self.gap(source, 'UNBOUND_DATASOURCE', 'No dataSource with this id and unambiguous source scope is configured.', span)
            return None
        dialect = config['dialect']
        if dialect not in self.rules['dialects'] or dialect not in sqlglot.Dialect.classes:
            self.gap(source, 'UNSUPPORTED_DIALECT', 'Explicit configured SQL dialect is unsupported.', span)
            return None
        identifier = self.entity(source, 'dataSource', dict(scopeId=config['scopeId'], configuredId=name), name,
                                 dict(dialect=dialect), span)
        return dict(id=identifier, name=name, dialect=dialect)

    @staticmethod
    def table_fields(data_source, table):
        return dict(dataSourceId=data_source['name'], table=table.name,
                    **({'catalog': table.catalog} if table.catalog else {}), **({'schema': table.db} if table.db else {}))

    @staticmethod
    def table_key(fields):
        return (fields['dataSourceId'], *(str(fields.get(key, '')).casefold() for key in ('catalog', 'schema', 'table')))

    def lookup_table(self, data_source, node):
        return self.tables.get(self.table_key(self.table_fields(data_source, node)))

    def parse_sql(self, source, text, data_source, span=None):
        try:
            return sqlglot.parse(text, read=data_source['dialect'], error_level=ErrorLevel.RAISE)
        except (ParseError, ValueError):
            self.gap(source, 'UNSUPPORTED_SQL', 'SQL AST parse failed; no table, column or query facts were fabricated.', span)
            return []

    def schemas(self):
        pending = []
        for source in self.input['sources']:
            if not source['path'].endswith('.sql') or not source['content'].strip():
                continue
            name = self.rules['sourceBindings'].get(source['path'])
            if not name:
                self.gap(source, 'UNBOUND_DATASOURCE', 'SQL source has no investigated explicit dataSource binding.')
                continue
            data_source = self.data_source(source, name)
            if not data_source:
                continue
            trees = self.parse_sql(source, source['content'], data_source)
            self.sources[source['id']] = (source, data_source, trees)
            for tree in trees:
                if not isinstance(tree, exp.Create):
                    continue
                if not isinstance(tree.this, exp.Schema) or not isinstance(tree.this.this, exp.Table):
                    self.gap(source, 'UNSUPPORTED_DDL', 'Only explicit CREATE TABLE column/key declarations are supported.')
                    continue
                schema = tree.this
                fields = self.table_fields(data_source, schema.this)
                key = self.table_key(fields)
                if key in self.tables:
                    self.gap(source, 'AMBIGUOUS_SCHEMA', 'Multiple declarations for this dataSource-qualified table.')
                    self.tables[key]['ambiguous'] = True
                    continue
                table_id = self.entity(source, 'dataTable', fields, schema.this.name, dict(dataSourceId=data_source['name']), basis='declared')
                self.relation(source, 'defines', data_source['id'], table_id, basis='declared')
                table = dict(id=table_id, fields=fields, columns={}, keys=[], source=source, ambiguous=False)
                self.tables[key] = table
                declarations = list(schema.expressions)
                for column in schema.expressions:
                    if not isinstance(column, exp.ColumnDef):
                        continue
                    constraints = [item.kind for item in column.constraints]
                    attributes = dict(dataType=column.kind.sql(dialect=data_source['dialect']),
                        nullable=False if any(isinstance(item, exp.NotNullColumnConstraint) for item in constraints) else 'unknown')
                    column_id = self.entity(source, 'dataColumn', dict(**fields, column=column.name), column.name, attributes, basis='declared')
                    table['columns'][column.name.casefold()] = column_id
                    self.relation(source, 'defines', data_source['id'], column_id, basis='declared')
                    if any(isinstance(item, exp.PrimaryKeyColumnConstraint) for item in constraints):
                        declarations.append(exp.PrimaryKey(expressions=[exp.to_identifier(column.name)]))
                for declaration in declarations:
                    if isinstance(declaration, exp.Constraint):
                        entries = declaration.expressions
                    else:
                        entries = [declaration]
                    for entry in entries:
                        if isinstance(entry, (exp.PrimaryKey, exp.ForeignKey, exp.UniqueColumnConstraint)):
                            pending.append((table, entry, data_source))
        for table, declaration, data_source in pending:
            source = table['source']
            nodes = declaration.this.expressions if isinstance(declaration.this, exp.Schema) else declaration.expressions
            names = [node.name for node in nodes]
            columns = [table['columns'].get(name.casefold()) for name in names]
            if not columns or not all(columns):
                self.gap(source, 'INCOMPLETE_KEY', 'DDL key does not name a complete declared column list.')
                continue
            key_type = 'foreign' if isinstance(declaration, exp.ForeignKey) else 'primary' if isinstance(declaration, exp.PrimaryKey) else 'unique'
            attributes = dict(keyType=key_type, columnIds=columns, orderedColumns=names)
            target = None
            if key_type == 'foreign':
                reference = declaration.args.get('reference')
                target_schema = reference.this if reference else None
                if isinstance(target_schema, exp.Schema):
                    target = self.lookup_table(data_source, target_schema.this)
                    right = [target['columns'].get(node.name.casefold()) for node in target_schema.expressions] if target else []
                    if not target or target['ambiguous'] or len(right) != len(columns) or not all(right):
                        target = None
                    else:
                        attributes['keyPairs'] = [dict(leftColumnId=left, rightColumnId=right, operator='equals') for left, right in zip(columns, right)]
                if not target:
                    self.gap(source, 'INCOMPLETE_KEY', 'Foreign key target/ordered columns are not fully declared in the same dataSource.')
            key_id = self.entity(source, 'dataKey', dict(**table['fields'], keyColumns=names), '.'.join(names), attributes, basis='declared')
            table['keys'].append((key_type, names, key_id))
            self.relation(source, 'defines', data_source['id'], key_id, basis='declared')
            if target:
                self.relation(source, 'maps_to', key_id, target['id'], basis='declared', refs=[self.cite(target['source'])])

    def query(self, source, text, data_source, semantic_key, span=None, caller=None, refs=()):
        trees = self.parse_sql(source, text, data_source, span)
        for ordinal, tree in enumerate(trees):
            if not isinstance(tree, exp.Select):
                if tree is not None:
                    self.gap(source, 'UNSUPPORTED_SQL', 'Only SELECT/CTE query implementations are collected; no business SQL is executed.', span)
                continue
            joins, read_ids = [], set()
            incomplete_joins = []

            def analyze(select, inherited):
                ctes = dict(inherited)
                with_clause = select.args.get('with_')
                if with_clause:
                    if with_clause.args.get('recursive'):
                        self.gap(source, 'UNSUPPORTED_SQL', 'Recursive CTE semantics are not collected.', span)
                        return {}
                    for cte in with_clause.expressions:
                        ctes[cte.alias.casefold()] = analyze(cte.this, ctes) if isinstance(cte.this, exp.Select) else {}
                aliases = {}

                def register(node):
                    if isinstance(node, exp.Table):
                        alias = node.alias_or_name.casefold()
                        if node.name.casefold() in ctes and not node.db and not node.catalog:
                            aliases[alias] = ctes[node.name.casefold()]
                        else:
                            table = self.lookup_table(data_source, node)
                            if not table or table['ambiguous']:
                                self.gap(source, 'UNDECLARED_TABLE', 'Query references a table without an unambiguous same-dataSource DDL declaration.', span)
                                aliases[alias] = {}
                            else:
                                aliases[alias] = dict(table['columns'])
                                read_ids.add(table['id'])
                    elif isinstance(node, exp.Subquery) and isinstance(node.this, exp.Select):
                        aliases[node.alias.casefold()] = analyze(node.this, ctes)
                    else:
                        self.gap(source, 'UNSUPPORTED_SQL', 'Dynamic table or unsupported FROM expression.', span)
                from_clause = select.args.get('from_')
                if from_clause:
                    register(from_clause.this)
                sql_joins = select.args.get('joins', [])
                for join in sql_joins:
                    register(join.this)

                def column(node):
                    if not isinstance(node, exp.Column) or isinstance(node.this, exp.Star):
                        return None
                    if node.table:
                        return aliases.get(node.table.casefold(), {}).get(node.name.casefold())
                    candidates = [mapping[node.name.casefold()] for mapping in aliases.values() if node.name.casefold() in mapping]
                    return candidates[0] if len(candidates) == 1 else None

                for join in sql_joins:
                    condition = join.args.get('on')
                    terms = list(condition.flatten()) if isinstance(condition, exp.And) else [condition] if condition else []
                    pairs = []
                    complete = bool(terms)
                    for term in terms:
                        left, right = (column(term.left), column(term.right)) if isinstance(term, exp.EQ) else (None, None)
                        if isinstance(term, exp.EQ) and ((left and isinstance(term.right, exp.Literal)) or (right and isinstance(term.left, exp.Literal))):
                            continue
                        if not left or not right:
                            complete = False
                        else:
                            pairs.append(dict(leftColumnId=left, rightColumnId=right, operator='equals'))
                    side, kind = join.side.lower(), join.kind.lower()
                    join_type = side if side in ('left', 'right', 'full') else 'cross' if kind == 'cross' else 'inner'
                    qualifier = dict(joinType=join_type, keyPairs=pairs, cardinality='unknown')
                    if condition:
                        qualifier['condition'] = condition.sql(dialect=data_source['dialect'])
                    # Do not hide a partially bound condition behind a confirmed joins edge.
                    key_complete = False
                    for table in self.tables.values():
                        for key_type, names, _ in table['keys']:
                            if key_type not in ('primary', 'unique'):
                                continue
                            key_ids = {table['columns'][name.casefold()] for name in names}
                            if any(key_ids <= {pair[side] for pair in pairs} for side in ('leftColumnId', 'rightColumnId')):
                                key_complete = True
                    if complete and pairs and key_complete:
                        joins.append((qualifier, pairs[0]['rightColumnId']))
                    else:
                        incomplete_joins.append(self.gap(source, 'INCOMPLETE_JOIN_KEY' if complete and pairs else 'UNRESOLVED_JOIN',
                            'Join has no fully bound ordered equality keys covering a declared unique/primary key; original condition remains in statement evidence.', span))
                # Only inspect this SELECT scope; nested scopes have their own alias maps.
                def walk(node):
                    if node is not select and isinstance(node, (exp.Select, exp.With)):
                        return
                    if isinstance(node, exp.Column) and not isinstance(node.this, exp.Star):
                        identifier = column(node)
                        if identifier:
                            read_ids.add(identifier)
                        else:
                            self.gap(source, 'UNRESOLVED_COLUMN', 'SQL column lacks an unambiguous declared or direct CTE projection binding.', span)
                    for child in node.iter_expressions():
                        walk(child)
                walk(select)
                projection = {}
                ambiguous = set()
                def project(name, identifier):
                    name = name.casefold()
                    if name in ambiguous:
                        return
                    if name in projection and projection[name] != identifier:
                        projection.pop(name)
                        ambiguous.add(name)
                        self.gap(source, 'UNRESOLVED_COLUMN', 'Repeated output names cannot form a unique downstream CTE binding.', span)
                    else:
                        projection[name] = identifier
                for expression in select.expressions:
                    name = expression.alias_or_name
                    node = expression.this if isinstance(expression, exp.Alias) else expression
                    if isinstance(node, exp.Star) or isinstance(node, exp.Column) and isinstance(node.this, exp.Star):
                        mappings = [aliases.get(node.table.casefold(), {})] if isinstance(node, exp.Column) and node.table else list(aliases.values())
                        if not mappings or any(not mapping for mapping in mappings):
                            self.gap(source, 'UNRESOLVED_COLUMN', 'Wildcard has an incomplete same-dataSource declaration or CTE scope.', span)
                        for mapping in mappings:
                            for output_name, identifier in mapping.items():
                                read_ids.add(identifier)
                                project(output_name, identifier)
                        continue
                    identifier = column(node)
                    if identifier and name:
                        project(name, identifier)
                return projection

            analyze(tree, {})
            identifier = self.query_entity(source, data_source, semantic_key + ':' + str(ordinal), span, caller, refs,
                dict(status='unknown', gapIds=incomplete_joins) if incomplete_joins else [item[0] for item in joins])
            for target in sorted(read_ids):
                self.relation(source, 'reads', identifier, target, span=span, refs=refs)
            for qualifier, target in joins:
                self.relation(source, 'joins', identifier, target, qualifier, span=span, refs=refs)

    def query_entity(self, source, data_source, semantic_key, span, caller, refs, joins):
        gap_id = self.gap(source, 'QUERY_SEMANTICS_NOT_COLLECTED', 'Business grain, valid states, time fields, NULL policy and units require independent semantic collection (T14).', span)
        unknown = dict(status='unknown', gapIds=[gap_id])
        data = source['content'].encode('utf-8')
        start, end = span or (0, len(data))
        attributes = dict(dialect=data_source['dialect'], statement=dict(startByte=start, endByte=end, text=data[start:end].decode('utf-8')),
            grainColumnIds=unknown, joins=joins, aggregates=unknown, validStates=unknown, timeFields=unknown, nullHandling=unknown, units=unknown)
        identifier = self.entity(source, 'queryImplementation', dict(scopeId=self.scope(source), semanticKey=semantic_key),
                                 semantic_key, attributes, span, refs=refs)
        if caller:
            self.relation(source, 'calls', caller, identifier, span=span, refs=refs)
        return identifier

    def orm(self):
        for source in self.input['sources']:
            if not source['path'].endswith(('.py', '.ts')) or not source['content'].strip():
                continue
            try:
                parsed = parse_source(source)
            except (SyntaxError, ValueError):
                self.gap(source, 'UNSUPPORTED_SYNTAX', 'Source failed parsing; no ORM bindings from this file are confirmed.')
                continue
            self.files[(source['repositoryId'], source['path'])] = (source, parsed)

        def binding(source, name, visited):
            key = (source['repositoryId'], source['path'], name)
            if key in visited:
                return None
            visited = {*visited, key}
            _, parsed = self.files[(source['repositoryId'], source['path'])]
            if name in parsed['bindings']:
                return (source, *parsed['bindings'][name], name) if parsed['bindings'][name] else None
            imported = parsed['imports'].get(name)
            if not imported:
                return None
            module, imported_name = imported
            module = imported_path(source['path'], module)
            if not module:
                return None
            candidates = [self.files[(source['repositoryId'], path)][0] for path in [module, module + '.ts', module + '/index.ts']
                          if (source['repositoryId'], path) in self.files]
            if len(candidates) != 1:
                return None
            provider = candidates[0]
            if provider['path'].endswith('.ts') and imported_name not in self.files[(provider['repositoryId'], provider['path'])][1]['exports']:
                return None
            return binding(provider, imported_name, visited)

        def instance(source, name):
            bound = binding(source, name, set())
            if not bound:
                return None
            provider, expression, span, _ = bound
            if expression[0] != 'call' or expression[1] != ('name', self.rules['ormObject']) or expression[2] != self.rules['connectMethod']:
                return None
            if len(expression[3]) != 1 or expression[3][0][0] != 'literal':
                self.gap(provider, 'DYNAMIC_INSTANCE', 'ORM instance dataSource id is not a static literal.', span)
                return None
            data_source = self.data_source(provider, expression[3][0][1], span)
            return (data_source, self.cite(provider, span)) if data_source else None

        def model(source, name):
            bound = binding(source, name, set())
            if not bound:
                return None
            provider, expression, span, canonical_name = bound
            if expression[0] != 'call' or expression[1][0] != 'name' or expression[2] != self.rules['modelMethod']:
                return None
            owner = instance(provider, expression[1][1])
            if not owner:
                self.gap(provider, 'UNBOUND_INSTANCE', 'Model receiver has no explicit lexical/imported ORM instance binding.', span)
                return None
            data_source, ref = owner
            args = expression[3]
            if len(args) != 2 or args[0][0] != 'literal' or args[1][0] != 'array' or any(item[0] != 'literal' for item in args[1][1]):
                self.gap(provider, 'UNSUPPORTED_ORM', 'Only investigated model(table, orderedPrimaryKeyColumns) mapping is supported.', span)
                return None
            table = self.lookup_table(data_source, exp.to_table(args[0][1]))
            names = [item[1] for item in args[1][1]]
            if not table or table['ambiguous'] or not any(kind == 'primary' and columns == names for kind, columns, _ in table['keys']):
                self.gap(provider, 'INCOMPLETE_MODEL_KEY', 'Model table or full ordered primary key disagrees with same-dataSource DDL.', span)
                return None
            identifier = self.entity(provider, 'symbol', dict(repositoryId=provider['repositoryId'], path=provider['path'], scopeId=self.scope(provider),
                lexicalPath=canonical_name, symbolKind='variable', normalizedSignature=''), canonical_name, dict(dataSourceId=data_source['name'], orderedKeyColumns=names), span)
            self.relation(provider, 'maps_to', identifier, table['id'], span=span, refs=[ref, self.cite(table['source'])])
            return table, identifier, data_source

        for source, parsed in self.files.values():
            for name in parsed['bindings']:
                model(source, name)
            for function in parsed['functions']:
                operation = None
                for ordinal, call in enumerate(function['calls']):
                    _, receiver, method, args, span = call
                    if receiver[0] != 'name':
                        self.gap(source, 'UNSUPPORTED_ORM', 'Dynamic/chained receiver is outside the investigated ORM pattern.', span)
                        continue
                    name = receiver[1]
                    if method not in (self.rules['queryMethod'], 'findAll', 'raw'):
                        continue
                    if name in function['blocked']:
                        self.gap(source, 'UNBOUND_INSTANCE', 'Function parameter/local binding shadows the ORM receiver.', span)
                        continue
                    if not operation:
                        operation = self.entity(source, 'operation', dict(repositoryId=source['repositoryId'], path=source['path'], scopeId=self.scope(source),
                            lexicalPath=function['name'], normalizedSignature=function['signature']), function['name'], span=function['span'])
                    if method == 'findAll':
                        target = model(source, name)
                        if target and not args:
                            refs = [*self.entities[target[1]]['assertion']['evidenceRefs'], self.cite(target[0]['source'])]
                            query_id = self.query_entity(source, target[2], source['repositoryId'] + ':' + source['path'] + ':' + function['name'] + ':orm:' + str(ordinal),
                                                         span, operation, refs, [])
                            self.relation(source, 'reads', query_id, target[0]['id'], span=span, refs=refs)
                        else:
                            self.gap(source, 'UNSUPPORTED_ORM', 'Model query has no proven complete model mapping or uses unsupported options.', span)
                        continue
                    owner = instance(source, name)
                    if not owner:
                        self.gap(source, 'UNBOUND_INSTANCE', 'Query receiver has no explicit local/imported dataSource binding.', span)
                        continue
                    if method != self.rules['queryMethod']:
                        self.gap(source, 'UNSUPPORTED_ORM', 'ORM method has not been investigated; no query facts are inferred.', span)
                        continue
                    sql = args[0] if len(args) == 1 else ('unknown',)
                    sql_refs = [owner[1]]
                    if sql[0] == 'name':
                        bound = binding(source, sql[1], set()) if sql[1] not in function['blocked'] else None
                        sql = bound[1] if bound else ('unknown',)
                        if bound:
                            sql_refs.append(self.cite(bound[0], bound[2]))
                    if sql[0] != 'literal':
                        self.gap(source, 'DYNAMIC_SQL', 'Query text is dynamic or has no static lexical/import binding.', span)
                        continue
                    self.query(source, sql[1], owner[0], source['repositoryId'] + ':' + source['path'] + ':' + function['name'] + ':query:' + str(ordinal),
                               span, operation, refs=sql_refs)

    def run(self):
        self.schemas()
        for source, data_source, trees in self.sources.values():
            if trees and not any(isinstance(tree, exp.Create) for tree in trees):
                self.query(source, source['content'], data_source, source['repositoryId'] + ':' + source['path'])
        self.orm()
        self.output['entities'] = list(self.entities.values())
        self.output['relations'] = list(self.relations.values())
        self.output['evidence'] = list(self.evidence.values())
        scopes = sorted({self.scope(source) for source in self.input['sources']})
        for scope in scopes:
            gaps = [item['id'] for item in self.output['gaps'] if item['scopeId'] == scope]
            self.output['coverage'].append(dict(capability='database', scopeId=scope,
                sourceSetId=self.input['sourceSets'][0]['selectorId'], status='partial' if gaps else 'complete' if self.entities else 'notConfigured',
                counts=dict(entities=len(self.entities), relations=len(self.relations)), gapIds=gaps))
        return self.output


def collect_database(payload, rules):
    return DatabaseCollector(payload, rules).run()
