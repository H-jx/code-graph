"""Collect existing query expressions and policy gaps. Never execute or rewrite SQL."""
import ast
import copy
import re

import sqlglot
from sqlglot import exp
from sqlglot.errors import ErrorLevel, ParseError

from semantic_facts import SemanticFacts
from source_ast import parse_source


def collect_report(payload, rules):
    facts = SemanticFacts(payload, rules, 'report')
    policy = next((item for item in payload['sources'] if item['path'] == rules['reportPolicy']), None)
    columns = [item for item in payload['entities'] if item['kind'] == 'dataColumn' and facts.refs(item)]
    by_source = {item['id']: item for item in payload['sources']}
    table_columns = {}
    for column in columns:
        fields = facts.fields(column)
        key = (fields['dataSourceId'], fields.get('catalog', ''), fields.get('schema', ''), fields['table'].casefold())
        table_columns.setdefault(key, {})[fields['column'].casefold()] = {column['entityId']}
    unique_sets = []
    for key in payload['entities']:
        if key['kind'] == 'dataKey':
            for assertion in key['assertions']:
                if assertion['basis'] != 'inferred' and facts.refs(key) and assertion['attributes'].get('keyType') in ('primary', 'unique'):
                    unique_sets.append(set(assertion['attributes']['columnIds']))

    def literal_sql(source, statement, refs):
        if source['path'].endswith('.sql'):
            return statement['text']
        text = statement['text']
        if source['path'].endswith('.py'):
            try:
                call = ast.parse(text.strip(), mode='eval').body
                if isinstance(call, ast.Call) and len(call.args) == 1 and isinstance(call.args[0], ast.Constant) and isinstance(call.args[0].value, str):
                    return call.args[0].value
            except SyntaxError:
                return None
        try:
            wrapped = dict(path='statement.ts', content='function statement() { return ' + text + '; }')
            calls = parse_source(wrapped)['functions'][0]['calls']
            sql = calls[0][3][0] if calls and len(calls[0][3]) == 1 else ('unknown',)
            if sql[0] == 'literal':
                return sql[1]
            if sql[0] == 'name':
                declarations = []
                for ref in refs:
                    provider = by_source[facts.evidence[ref]['sourceId']]
                    parsed = parse_source(provider)
                    bound = parsed['bindings'].get(sql[1])
                    if bound and bound[0][0] == 'literal':
                        declarations.append(bound[0][1])
                if len(set(declarations)) == 1:
                    return declarations[0]
        except (ValueError, SyntaxError, IndexError):
            return None
        return None

    for query in [item for item in payload['entities'] if item['kind'] == 'queryImplementation']:
        refs = facts.refs(query)
        assertions = [item for item in query['assertions'] if item['basis'] != 'inferred' and 'statement' in item['attributes']]
        if not assertions:
            continue
        attributes = copy.deepcopy(assertions[0]['attributes'])
        statement = attributes['statement']
        candidates = []
        for ref in refs:
            evidence = facts.evidence[ref]
            source = by_source[evidence['sourceId']]
            span = (statement['startByte'], statement['endByte'])
            if evidence['startByte'] <= span[0] < span[1] <= evidence['endByte'] and source['content'].encode('utf-8')[span[0]:span[1]].decode('utf-8') == statement.get('text'):
                candidates.append(source)
        candidates = {item['id']: item for item in candidates}
        if len(candidates) != 1:
            if policy:
                facts.gap(policy, 'REPORT_SOURCE_AMBIGUOUS', 'Query statement cannot be bound to one original source range.')
            continue
        source = next(iter(candidates.values()))
        sql = literal_sql(source, statement, refs)
        if not sql or attributes['dialect'] != 'sqlite':
            facts.gap(source, 'REPORT_IMPLEMENTATION_UNSUPPORTED', 'No supported static SELECT expression was recoverable from this SQL/ORM implementation.', span)
            continue
        try:
            trees = sqlglot.parse(sql, read='sqlite', error_level=ErrorLevel.RAISE)
            ordinal = int(facts.fields(query)['semanticKey'].rsplit(':', 1)[1])
            tree = trees[ordinal]
            if not isinstance(tree, exp.Select):
                raise ValueError('not SELECT')
        except (ValueError, ParseError, IndexError):
            facts.gap(source, 'REPORT_IMPLEMENTATION_UNSUPPORTED', 'Query AST is unsupported; no field or business policy is invented.', span)
            continue
        reads = [relation for relation in payload['relations'] if relation['kind'] == 'reads' and relation['fromId'] == query['entityId']]
        data_sources = {facts.fields(facts.by_id[relation['toId']]).get('dataSourceId') for relation in reads
                        if facts.by_id[relation['toId']]['kind'] in ('dataTable', 'dataColumn', 'dataKey')}
        if len(data_sources) != 1:
            facts.gap(source, 'REPORT_DATASOURCE_UNKNOWN', 'Existing query reads do not prove one dataSource.', span)
            continue
        data_source = next(iter(data_sources))
        aggregates, groupings, derived_unique = [], [], []
        unresolved = False

        def analyze(select, inherited):
            ctes = dict(inherited)
            with_clause = select.args.get('with_')
            if with_clause:
                if with_clause.args.get('recursive'):
                    facts.gap(source, 'REPORT_EXPRESSION_UNSUPPORTED', 'Recursive CTE lineage is unsupported.', span)
                    return []
                for cte in with_clause.expressions:
                    ctes[cte.alias.casefold()] = {name.casefold(): ids for name, _, ids in analyze(cte.this, ctes)} if isinstance(cte.this, exp.Select) else {}
            aliases = {}
            def register(node):
                if isinstance(node, exp.Table):
                    if node.name.casefold() in ctes and not node.db and not node.catalog:
                        aliases[node.alias_or_name.casefold()] = ctes[node.name.casefold()]
                    else:
                        aliases[node.alias_or_name.casefold()] = table_columns.get((data_source, node.catalog, node.db, node.name.casefold()), {})
                elif isinstance(node, exp.Subquery) and isinstance(node.this, exp.Select):
                    aliases[node.alias.casefold()] = {name.casefold(): ids for name, _, ids in analyze(node.this, ctes)}
            if select.args.get('from_'):
                register(select.args['from_'].this)
            for join in select.args.get('joins', []):
                register(join.this)
            def lineage(expression):
                nonlocal unresolved
                result = set()
                for column in expression.find_all(exp.Column):
                    if isinstance(column.this, exp.Star):
                        unresolved = True
                        facts.gap(source, 'REPORT_EXPRESSION_UNSUPPORTED', 'Wildcard output requires an explicit supported projection.', span)
                        continue
                    matches = [mapping[column.name.casefold()] for alias, mapping in aliases.items()
                               if (not column.table or alias == column.table.casefold()) and column.name.casefold() in mapping]
                    if len(matches) == 1:
                        result.update(matches[0])
                    else:
                        unresolved = True
                        facts.gap(source, 'REPORT_EXPRESSION_UNSUPPORTED', 'Expression has an unresolved or ambiguous source column.', span)
                return result
            group = select.args.get('group')
            if group:
                ids = set().union(*(lineage(item) for item in group.expressions))
                groupings.append(ids)
                if len(ids) == len(group.expressions) and all(isinstance(item, exp.Column) for item in group.expressions):
                    derived_unique.append(ids)
            projection = []
            for expression in select.expressions:
                raw = expression.this if isinstance(expression, exp.Alias) else expression
                name = expression.alias_or_name
                ids = lineage(raw)
                if isinstance(raw, exp.Star) or not name:
                    facts.gap(source, 'REPORT_EXPRESSION_UNSUPPORTED', 'Output needs an explicit column name or alias.', span)
                    continue
                projection.append((name, raw, ids))
                for aggregate in raw.find_all(exp.AggFunc):
                    aggregates.append(dict(function=aggregate.key.upper(), expression=aggregate.sql(dialect='sqlite'), sourceColumnIds=sorted(lineage(aggregate))))
            return projection

        projection = analyze(tree, {})
        policy_refs = [facts.cite(policy)] if policy else []
        policy_text = policy['content'] if policy else ''
        grain_match = re.search(r'^Grain: ([a-z_ +]+),', policy_text, re.M)
        grain_names = [item.strip() for item in grain_match.group(1).split('+')] if grain_match else []
        grain_ids = [item['entityId'] for name in grain_names for item in columns if facts.fields(item).get('dataSourceId') == data_source
                     and facts.fields(item).get('table') == rules['reportBaseTable'] and facts.fields(item).get('column') == name]
        policy_bound = bool(policy) and data_source == rules['dataSourceId']
        grain_ok = policy_bound and not unresolved and bool(grain_ids) and len(grain_ids) == len(grain_names)
        if grain_names and any(not set(grain_names) <= {facts.fields(facts.by_id[identifier])['column'] for identifier in group} for group in groupings):
            facts.gap(source, 'PREAGGREGATION_GRAIN_MISMATCH', 'A grouping omits required tenant/document/line grain keys.', span)
            grain_ok = False
        if not isinstance(attributes['joins'], list):
            facts.gap(source, 'REPORT_JOIN_KEYS_UNKNOWN', 'Existing query joins have incomplete or unsupported ordered key bindings.', span)
            grain_ok = False
        if tree.args.get('group'):
            root_names = {item.name for item in tree.args['group'].expressions if isinstance(item, exp.Column)}
            grain_ok &= set(grain_names) <= root_names
        children = [join for join in tree.args.get('joins', []) if isinstance(join.this, exp.Table)]
        has_root_aggregates = any(isinstance(item, exp.AggFunc) for expression in tree.expressions for item in expression.walk())
        if has_root_aggregates and len(children) > 1 and any(join.this.name not in {cte.alias for cte in tree.ctes} for join in children):
            facts.gap(source, 'JOIN_FANOUT_RISK', 'Multiple child relations are joined before outer aggregation; amounts may multiply.', span)
            grain_ok = False
        where = tree.args.get('where')
        child_aliases = {join.this.alias_or_name for join in children if join.side.upper() == 'LEFT'}
        if where and any(column.table in child_aliases for column in where.find_all(exp.Column)):
            facts.gap(source, 'CHILD_FILTER_AFTER_LEFT_JOIN', 'WHERE filters the nullable side of a LEFT JOIN and may drop no-match detail rows.', span)
            grain_ok = False
        attributes['grainColumnIds'] = grain_ids if grain_ok else facts.unknown(source, 'REPORT_GRAIN_UNKNOWN', 'Full policy grain/tenant keys are missing, unsafe or not bound to this dataSource.', span)
        if not policy_bound:
            facts.gap(source, 'REPORT_POLICY_MISSING', 'No original business policy is bound to this query/dataSource.', span)
        attributes['aggregates'] = aggregates if not unresolved else facts.unknown(source, 'REPORT_AGGREGATE_SOURCE_UNKNOWN', 'Aggregate inputs contain unresolved columns.', span)
        states = sorted({e.right.this for e in tree.find_all(exp.EQ) if isinstance(e.left, exp.Column) and e.left.name == 'status' and isinstance(e.right, exp.Literal)})
        expected_states = {state for pair in re.findall(r'\b([A-Z][A-Z_]+) purchases\b|status ([A-Z][A-Z_]+)', policy_text) for state in pair if state} if policy_bound else set()
        parent_state = re.search(r'\b([A-Z][A-Z_]+) purchases\b', policy_text)
        child_state = re.search(r'status ([A-Z][A-Z_]+)', policy_text)
        not_deleted = re.search(r'\b([a-z_]+)\s*=\s*0', policy_text)
        filters_proven = policy_bound
        def conjuncts(expression):
            if isinstance(expression, exp.Paren):
                return conjuncts(expression.this)
            if isinstance(expression, exp.And):
                return [*conjuncts(expression.left), *conjuncts(expression.right)]
            return [expression]
        for select in tree.find_all(exp.Select):
            tables = []
            if select.args.get('from_') and isinstance(select.args['from_'].this, exp.Table):
                tables.append(select.args['from_'].this)
            tables.extend(join.this for join in select.args.get('joins', []) if isinstance(join.this, exp.Table))
            declared = [(table, table_columns.get((data_source, table.catalog, table.db, table.name.casefold()), {})) for table in tables]
            conditions = conjuncts(select.args['where'].this) if select.args.get('where') else []
            for join in select.args.get('joins', []):
                if join.args.get('on'):
                    conditions.extend(conjuncts(join.args['on']))
            def mandatory(alias, field, value, numeric=False):
                unique_column = sum(field in columns for _, columns in declared) == 1
                return any(isinstance(term, exp.EQ) and isinstance(term.left, exp.Column)
                    and term.left.name.casefold() == field and (term.left.table == alias or not term.left.table and unique_column)
                    and isinstance(term.right, exp.Literal) and term.right.this == value
                    and term.right.is_string == (not numeric) for term in conditions)
            for table, declared_columns in declared:
                if 'status' in declared_columns:
                    wanted = parent_state if table.name == rules['reportBaseTable'] else child_state
                    if wanted and not mandatory(table.alias_or_name, 'status', wanted.group(1)):
                        filters_proven = False
                if not_deleted and not_deleted.group(1) in declared_columns and not mandatory(table.alias_or_name, not_deleted.group(1), '0', True):
                    filters_proven = False
        if policy_bound and not filters_proven:
            facts.gap(source, 'REPORT_VALID_FILTER_UNKNOWN', 'Required state/deletion filters are absent or not mandatory conjuncts; OR/name coincidence does not prove validity.', span)
        attributes['validStates'] = states if filters_proven and expected_states and expected_states == set(states) else facts.unknown(source, 'REPORT_STATES_UNKNOWN', 'SQL state/deletion filters do not cover the independently declared policy.', span)
        joins = attributes['joins']
        if isinstance(joins, list):
            for join in joins:
                left = {pair['leftColumnId'] for pair in join['keyPairs']}
                right = {pair['rightColumnId'] for pair in join['keyPairs']}
                if any(key <= left for key in [*unique_sets, *derived_unique]) and any(key <= right for key in [*unique_sets, *derived_unique]):
                    join['cardinality'] = 'oneToOne'
                else:
                    join['cardinality'] = 'unknown'
                    facts.gap(source, 'REPORT_CARDINALITY_UNKNOWN', 'This join has no proof that both written key sides are unique; no 1:1 relationship is assumed.', span)
        coalesces = list(tree.find_all(exp.Coalesce))
        zero_fallback = coalesces and all(item.expressions and isinstance(item.expressions[-1], exp.Literal)
            and not item.expressions[-1].is_string and item.expressions[-1].this == '0' for item in coalesces)
        attributes['nullHandling'] = 'COALESCE to zero after LEFT JOIN' if policy_bound and 'No match is zero' in policy_text and zero_fallback else facts.unknown(source, 'REPORT_NULL_POLICY_UNKNOWN', 'No matching original NULL policy and static zero fallback were proven.', span)
        attributes['timeFields'] = facts.unknown(source, 'REPORT_TIME_FIELD_UNKNOWN', 'The original policy does not identify a report time field.', span)
        attributes['units'] = [dict(field=name, unit='cent') for name, _, _ in projection if name.endswith('Cents') and name in policy_text] if policy_bound else facts.unknown(source, 'REPORT_UNIT_UNKNOWN', 'No original unit policy is bound to this dataSource.', span)
        if policy_bound:
            metrics = set(re.findall(r'\b([A-Za-z][A-Za-z0-9_]*)\s*=', policy_text))
            missing_units = metrics & {name for name, _, _ in projection} - {unit['field'] for unit in attributes['units']}
            if missing_units:
                facts.gap(source, 'REPORT_UNIT_UNKNOWN', 'Original policy does not declare physical units for: ' + ', '.join(sorted(missing_units)), span)
        fields = facts.fields(query)
        query_id = facts.entity(source, 'queryImplementation', fields, fields['semanticKey'], attributes, span, basis='declared' if policy_bound else 'observed', refs=[*refs, *policy_refs])
        for name, expression, ids in projection:
            field_id = facts.entity(source, 'reportField', dict(scopeId=query['scopeId'], semanticKey=fields['semanticKey'] + ':field:' + name), name,
                dict(queryId=query_id, outputName=name, sourceColumnIds=sorted(ids) if not unresolved and (ids or not any(expression.find_all(exp.Column))) else facts.unknown(source, 'REPORT_FIELD_SOURCE_UNKNOWN', 'Field source columns could not be fully bound.', span),
                     expression=expression.sql(dialect='sqlite'), transforms=sorted({item.key for item in expression.walk() if isinstance(item, (exp.AggFunc, exp.Coalesce, exp.Sub, exp.Add, exp.Mul))})), span, refs=[*refs, *policy_refs])
            facts.relation(source, 'references', query_id, field_id, span=span, refs=refs)
            for identifier in sorted(ids):
                facts.relation(source, 'references', field_id, identifier, span=span, refs=[*refs, *facts.refs(facts.by_id[identifier])])
    return facts.finish()
