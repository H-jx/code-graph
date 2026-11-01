"""Protocol output for the two semantic stages; all citations refer to original bytes."""
import json
import pathlib

from identity import evaluate, stable_id


class SemanticFacts:
    def __init__(self, payload, rules, capability):
        self.input, self.rules, self.capability = payload, rules, capability
        self.project = payload['configSubset']['projectId']
        self.specs = {item['kind']: item['identityComponents'] for item in json.loads(
            pathlib.Path(__file__).with_name('identity-spec.json').read_text(encoding='utf-8'))['entityKinds']}
        self.output = dict(protocolVersion='1.0', runId=payload['runId'], stageId=payload['stageId'],
            inputSnapshotId=payload['inputSnapshotId'], entities=[], relations=[], evidence=[], dependencies=[], coverage=[], gaps=[])
        self.entities, self.relations, self.evidence = {}, {}, {}
        self.original_evidence = {item['id']: item for item in payload.get('evidence', [])}
        self.by_id = {item['entityId']: item for item in payload['entities']}
        self.sources = {(item['repositoryId'], item['path']): item for item in payload['sources']}

    def scope(self, source):
        scopes = [item['id'] for item in self.input['configSubset']['scopes'] if source['repositoryId'] in item['repositoryIds']]
        return scopes[0] if len(scopes) == 1 else 'repo.' + source['repositoryId']

    def cite(self, source, span=None):
        data = source['content'].encode('utf-8')
        start, end = span or (0, len(data))
        identifier = stable_id('ev_', [source['id'], source['contentHash'], start, end, 'semantic'])
        self.evidence[identifier] = dict(id=identifier, sourceId=source['id'], contentHash=source['contentHash'],
            startByte=start, endByte=end, startLine=data[:start].count(b'\n') + 1,
            endLine=data[:max(start, end - 1)].count(b'\n') + 1, role='semantic')
        return identifier

    def refs(self, item):
        refs = []
        for assertion in item.get('assertions', []):
            if assertion['basis'] == 'inferred' or assertion['origin'] == 'ai':
                continue
            for ref in assertion['evidenceRefs']:
                if ref in self.original_evidence:
                    self.evidence[ref] = self.original_evidence[ref]
                    refs.append(ref)
        return list(dict.fromkeys(refs))

    def gap(self, source, code, reason, span=None):
        identifier = stable_id('gap_', [self.cite(source, span), code, reason])
        if not any(item['id'] == identifier for item in self.output['gaps']):
            self.output['gaps'].append(dict(id=identifier, code=code, capability=self.capability, scopeId=self.scope(source),
                sourceRefs=[source['id']], candidateIds=[], reason=reason,
                requiredEvidence=['original business policy, complete ordered keys and supported static expressions']))
        return identifier

    def unknown(self, source, code, reason, span=None):
        return dict(status='unknown', gapIds=[self.gap(source, code, reason, span)])

    @staticmethod
    def fields(entity):
        return {slot['name']: slot['value'] for slot in entity['identity'] if 'value' in slot}

    def entity(self, source, kind, fields, name, attributes=None, span=None, basis='observed', refs=()):
        slots = [dict(name=spec['name'], **({'value': fields[spec['name']]} if spec['name'] in fields else {})) for spec in self.specs[kind]]
        identifier = evaluate(dict(type='entity', projectId=self.project, kind=kind, slots=slots), self.specs)
        if identifier not in self.entities:
            self.entities[identifier] = dict(entityId=identifier, kind=kind, scopeId=fields.get('scopeId', self.scope(source)), identity=slots,
                assertion=dict(producerKey=self.input['pluginId'], name=name, attributes=attributes or {}, basis=basis, origin='static',
                               evidenceRefs=list(dict.fromkeys([self.cite(source, span), *refs]))))
        return identifier

    def relation(self, source, kind, origin, target, qualifiers=None, span=None, basis='observed', refs=()):
        qualifiers = qualifiers or {}
        identifier = stable_id('rel_', [self.project, kind, origin, target, qualifiers])
        self.relations[identifier] = dict(relationId=identifier, kind=kind, fromId=origin, toId=target, qualifiers=qualifiers, resolution='confirmed',
            assertion=dict(producerKey=self.input['pluginId'], attributes={}, basis=basis, origin='static',
                           evidenceRefs=list(dict.fromkeys([self.cite(source, span), *refs]))))

    def finish(self):
        self.output.update(entities=list(self.entities.values()), relations=list(self.relations.values()), evidence=list(self.evidence.values()))
        for scope in sorted({self.scope(source) for source in self.input['sources']}):
            gaps = [item['id'] for item in self.output['gaps'] if item['scopeId'] == scope]
            self.output['coverage'].append(dict(capability=self.capability, scopeId=scope,
                sourceSetId=self.input['sourceSets'][0]['selectorId'], status='partial' if gaps else 'complete' if self.entities else 'notConfigured',
                counts=dict(entities=len(self.entities), relations=len(self.relations)), gapIds=gaps))
        return self.output
