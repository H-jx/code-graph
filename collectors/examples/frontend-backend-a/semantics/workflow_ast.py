"""Interpret only documented transitions and actual state assignments in the example grammar."""
import re

from tree_sitter import Language, Parser
import tree_sitter_typescript

from semantic_facts import SemanticFacts
from source_ast import decode_js_literal, descendants, typescript_signature


def collect_workflow(payload, rules):
    facts = SemanticFacts(payload, rules, 'workflow')
    code = next((item for item in payload['sources'] if item['path'] == rules['workflowCode']), None)
    document = next((item for item in payload['sources'] if item['path'] == rules['workflowDocument']), None)
    if not document and not code:
        return facts.finish()
    scope = facts.scope(document or code)
    if document and code and facts.scope(code) != scope:
        facts.gap(document, 'WORKFLOW_SCOPE_UNKNOWN', 'Document and code have no explicit same-workflow scope binding.')
        code = None
    transitions = []
    offset = 0
    for line in (document['content'] if document else '').splitlines(keepends=True):
        span = (offset, offset + len(line.rstrip('\r\n').encode('utf-8')))
        offset += len(line.encode('utf-8'))
        match = re.fullmatch(r'([A-Z_]+) -> ([A-Z_]+): ([a-z]+)(?: or ([a-z]+))?, (.+)\.', line.strip())
        if match:
            before, after, first, second, guard = match.groups()
            for trigger in [first, second] if second else [first]:
                transitions.append(dict(before=before, after=after, trigger=trigger, guard=guard, span=span))
    if not transitions:
        facts.gap(document or code, 'WORKFLOW_EVIDENCE_MISSING', 'No supported original transition declarations were found; code writes/calls alone are not business ordering.')
    workflow = rules['workflowId']
    states = {}
    for transition in transitions:
        for name in [transition['before'], transition['after']]:
            states[name] = facts.entity(document, 'workflowState', dict(scopeId=scope, workflowId=workflow, semanticKey=name), name,
                                        dict(workflowId=workflow, state=name), transition['span'], basis='declared')
    functions = {}
    if code:
        data = code['content'].encode('utf-8')
        tree = Parser(Language(tree_sitter_typescript.language_typescript())).parse(data)
        if tree.root_node.has_error:
            facts.gap(code, 'UNSUPPORTED_SYNTAX', 'Workflow source failed parsing; no code transition evidence is added.')
        else:
            text = lambda node: data[node.start_byte:node.end_byte].decode('utf-8')
            for declaration in tree.root_node.named_children:
                node = declaration.child_by_field_name('declaration') if declaration.type == 'export_statement' else declaration
                if node is None or node.type != 'function_declaration':
                    continue
                name = text(node.child_by_field_name('name'))
                parameters = node.child_by_field_name('parameters')
                operation = facts.entity(code, 'operation', dict(repositoryId=code['repositoryId'], path=code['path'], scopeId=scope,
                    lexicalPath=name, normalizedSignature=typescript_signature(parameters)), name,
                    dict(workflowId=workflow), (node.start_byte, node.end_byte))
                writes, calls = [], []
                blocked = {text(part) for item in parameters.named_children for part in descendants(item) if part.type == 'identifier'}
                parameter_names = set()
                for parameter in parameters.named_children:
                    pattern = parameter.child_by_field_name('pattern') or parameter
                    if pattern.type == 'identifier':
                        parameter_names.add(text(pattern))
                state_parameter = rules['stateParameter']
                rebound = any(part.type == 'assignment_expression' and part.child_by_field_name('left').type == 'identifier'
                    and text(part.child_by_field_name('left')) == state_parameter for part in descendants(node.child_by_field_name('body')))
                def inspect(child, guards):
                    if child is not node and child.type in ('function_declaration', 'arrow_function', 'function_expression'):
                        local_name = child.child_by_field_name('name')
                        if local_name:
                            blocked.add(text(local_name))
                        return
                    if child.type == 'variable_declarator':
                        local_name = child.child_by_field_name('name')
                        if local_name:
                            blocked.update(text(part) for part in descendants(local_name) if part.type == 'identifier')
                    if child.type == 'call_expression':
                        target = child.child_by_field_name('function')
                        if target.type == 'identifier':
                            calls.append((text(target), (child.start_byte, child.end_byte)))
                    if child.type == 'if_statement':
                        condition = child.child_by_field_name('condition')
                        consequence = child.child_by_field_name('consequence')
                        if consequence:
                            inspect(consequence, [*guards, text(condition)])
                        alternative = child.child_by_field_name('alternative')
                        if alternative:
                            inspect(alternative, [*guards, '!(' + text(condition) + ')'])
                        return
                    if child.type == 'assignment_expression':
                        left, right = child.child_by_field_name('left'), child.child_by_field_name('right')
                        if left.type == 'member_expression' and text(left.child_by_field_name('property')) == rules['stateProperty']:
                            owner = left.child_by_field_name('object')
                            if owner.type != 'identifier' or text(owner) != state_parameter or state_parameter not in parameter_names or rebound:
                                facts.gap(code, 'WORKFLOW_STATE_OWNER_UNKNOWN', 'State write does not have an unchanged explicit workflow parameter owner.', (child.start_byte, child.end_byte))
                                return
                            state = decode_js_literal(text(right))
                            if state:
                                writes.append((state, ' AND '.join(guards) if guards else None, (child.start_byte, child.end_byte)))
                            else:
                                facts.gap(code, 'DYNAMIC_WORKFLOW_STATE', 'State assignment is dynamic.', (child.start_byte, child.end_byte))
                    for part in child.named_children:
                        inspect(part, guards)
                # Dominating failure guards remain raw expressions, never evaluated.
                preconditions = []
                for statement in node.child_by_field_name('body').named_children:
                    consequence = statement.child_by_field_name('consequence') if statement.type == 'if_statement' else None
                    throws = consequence is not None and (consequence.type == 'throw_statement' or
                        consequence.type == 'statement_block' and len(consequence.named_children) == 1 and consequence.named_children[0].type == 'throw_statement')
                    if throws:
                        preconditions.append('!(' + text(statement.child_by_field_name('condition')) + ')')
                    else:
                        inspect(statement, preconditions)
                functions[name] = None if name in functions else dict(id=operation, writes=writes, calls=calls, blocked=blocked, span=(node.start_byte, node.end_byte))
                for state, guard, span in writes:
                    state_id = facts.entity(code, 'workflowState', dict(scopeId=scope, workflowId=workflow, semanticKey=state), state,
                        dict(workflowId=workflow, state=state), span)
                    facts.relation(code, 'triggers', operation, state_id, dict(guard=guard) if guard else {}, span,
                                   refs=[facts.cite(code, (node.start_byte, node.end_byte))])
    for function in functions.values():
        if not function:
            continue
        for target, span in function['calls']:
            if target not in function['blocked'] and functions.get(target):
                facts.relation(code, 'calls', function['id'], functions[target]['id'], span=span, refs=[facts.cite(code, functions[target]['span'])])
    if not transitions:
        return facts.finish()
    for transition in transitions:
        function = functions.get(transition['trigger'])
        qualifiers = dict(trigger=transition['trigger'], guard=transition['guard'], keyPairs=[])
        if function:
            qualifiers['operationId'] = function['id']
        facts.relation(document, 'transitions_to', states[transition['before']], states[transition['after']], qualifiers,
                       transition['span'], basis='declared')
        writes = [write for write in function['writes'] if write[0] == transition['after']] if function else []
        if not writes:
            facts.gap(document, 'WORKFLOW_IMPLEMENTATION_MISSING', 'Declared transition has no matching supported code state write.', transition['span'])
        for state, guard, span in writes:
            qualifiers = dict(operationId=function['id'], trigger=transition['trigger'], keyPairs=[])
            if guard:
                qualifiers['guard'] = guard
            else:
                facts.gap(code, 'WORKFLOW_GUARD_UNKNOWN', 'State write has no collected guard; document condition is a separate assertion.', span)
            facts.relation(code, 'transitions_to', states[transition['before']], states[state], qualifiers, span,
                           basis='declared', refs=[facts.cite(document, transition['span']), facts.cite(code, function['span'])])
    conflict = next((line for line in document['content'].splitlines() if line.startswith('Conflict variant:')), None)
    if conflict:
        facts.gap(document, 'CONFLICTING_EVIDENCE', conflict)
        finish = next((item for item in transitions if item['trigger'] == 'finish'), None)
        if finish:
            facts.relation(document, 'transitions_to', states[finish['before']], states[finish['after']],
                dict(trigger='finish', guard=conflict, keyPairs=[]), basis='declared')
    # These predecessors are declared in the business document, not inferred from calls.
    pages = re.search(r'UI pages: (/.+?) precedes (/.+?), which precedes (/.+?)\.', document['content'])
    if pages:
        ids = [facts.entity(document, 'documentType', dict(scopeId=scope, semanticKey='page:' + page), page,
                            dict(pageRoute=page), basis='declared') for page in pages.groups()]
        for left, right in zip(ids, ids[1:]):
            facts.relation(document, 'precedes', left, right, basis='declared')
    documents = re.search(r'Documents ([a-z]+) -> ([a-z]+) -> ([a-z]+) use ordered keys ([a-z_, ]+)\.', document['content'])
    if documents:
        names = documents.groups()[:3]
        columns = [item.strip() for item in documents.group(4).split(',')]
        ids = {name: facts.entity(document, 'documentType', dict(scopeId=scope, semanticKey='document:' + name), name,
                                  dict(orderedKeyNames=columns), basis='declared') for name in names}
        data_columns = {(facts.fields(item).get('dataSourceId'), facts.fields(item).get('table'), facts.fields(item).get('column')): item
                        for item in payload['entities'] if item['kind'] == 'dataColumn' and facts.refs(item)}
        for left, right in zip(names, names[1:]):
            pairs, refs = [], []
            for column in columns:
                before = data_columns.get((rules['dataSourceId'], rules['documentTables'][left], column))
                after = data_columns.get((rules['dataSourceId'], rules['documentTables'][right], column))
                if before and after:
                    pairs.append(dict(leftColumnId=before['entityId'], rightColumnId=after['entityId'], operator='equals'))
                    refs.extend([*facts.refs(before), *facts.refs(after)])
            if len(pairs) == len(columns) and pairs:
                facts.relation(document, 'relates_document', ids[left], ids[right], dict(keyPairs=pairs), basis='declared', refs=refs)
            else:
                facts.gap(document, 'DOCUMENT_KEYS_MISSING', 'Declared ordered document keys are not completely bound to same-dataSource columns.')
    facts.gap(document, 'WORKFLOW_EVIDENCE_MISSING', 'Logging/call order and navigation links do not establish additional business successors.')
    return facts.finish()
