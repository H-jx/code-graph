// Join only exported facts and their original binding evidence. No project paths or names.
import { createHash } from 'node:crypto';
import { computeRelationId } from './identity.mjs';

export function linkFacts(input) {
  const output = {
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
  const entities = new Map(input.entities.map((entity) => [entity.entityId, entity]));
  const evidence = new Map((input.evidence ?? []).map((item) => [item.id, item]));
  const emittedEvidence = new Map(),
    emittedRelations = new Map();
  const fields = (entity) =>
    Object.fromEntries(entity.identity.filter((s) => 'value' in s).map((s) => [s.name, s.value]));
  const original = (assertion) =>
    assertion.basis !== 'inferred' &&
    assertion.origin !== 'ai' &&
    assertion.resolution !== 'candidate' &&
    assertion.evidenceRefs.length > 0 &&
    assertion.evidenceRefs.every((id) => evidence.has(id));
  const proofs = (item) =>
    item.assertions.filter(original).flatMap((assertion) => assertion.evidenceRefs);
  const edges = (kind, from, to) =>
    input.relations.filter(
      (relation) =>
        relation.kind === kind &&
        (!from || relation.fromId === from) &&
        (!to || relation.toId === to) &&
        proofs(relation).length,
    );
  function gap(scope, owner, code, reason, candidates = []) {
    const id =
      'gap_' +
      createHash('sha256')
        .update(JSON.stringify([scope, owner, code, candidates]))
        .digest('hex');
    if (output.gaps.some((g) => g.id === id)) return;
    const sourceRefs = [
      ...new Set(
        proofs(entities.get(owner) ?? { assertions: [] }).map((id) => evidence.get(id).sourceId),
      ),
    ];
    output.gaps.push({
      id,
      code,
      capability: 'link',
      scopeId: scope,
      sourceRefs,
      candidateIds: candidates,
      reason,
      requiredEvidence: [
        'complete candidate set and original call/registration/instance binding; an equal name alone is insufficient',
      ],
    });
  }
  const humanProof = (items) =>
    items.some((item) =>
      item.assertions.some(
        (assertion) =>
          original(assertion) && assertion.basis === 'declared' && assertion.origin === 'human',
      ),
    );
  function relation(kind, from, to, refs, confirmed, human = false) {
    refs = [...new Set(refs)].filter((id) => evidence.has(id));
    if (!refs.length) return;
    const relationId = computeRelationId(input.configSubset.projectId, kind, from, to, {});
    refs.forEach((id) => emittedEvidence.set(id, evidence.get(id)));
    const previous = emittedRelations.get(relationId);
    const assertion = {
      producerKey: input.pluginId,
      attributes: {},
      basis: confirmed ? (human ? 'declared' : 'observed') : 'inferred',
      origin: human ? 'human' : 'static',
      evidenceRefs: refs,
    };
    // Conflicting producer assertions never upgrade an already ambiguous edge.
    emittedRelations.set(relationId, {
      relationId,
      kind,
      fromId: from,
      toId: to,
      qualifiers: {},
      resolution: confirmed && previous?.resolution !== 'candidate' ? 'confirmed' : 'candidate',
      assertion,
    });
  }
  const config = input.configSubset;
  function requestBinding(request, attributes) {
    const bindings = config.bindings.filter(
      (binding) =>
        binding.kind === 'clientToService' &&
        binding.scopeId === request.scopeId &&
        binding.from === attributes.clientName,
    );
    const service = config.services.find((service) => service.id === attributes.serviceId);
    let proven =
      attributes.bindingComplete === true &&
      bindings.length === 1 &&
      bindings[0].to === service?.id &&
      service.baseUrls.includes(attributes.baseURL);
    let path = attributes.path;
    if (attributes.proxy) {
      const proxy = attributes.proxy;
      const proxyBindings = config.bindings.filter(
        (binding) =>
          binding.kind === 'proxyToService' &&
          binding.scopeId === request.scopeId &&
          binding.from === proxy.target &&
          binding.to === service?.id,
      );
      proven &&=
        proxyBindings.length === 1 &&
        proxy.prefix === attributes.baseURL &&
        typeof proxy.rewrite === 'string' &&
        typeof proxy.target === 'string';
      if (proven && path) path = proxy.rewrite.replace(/\/$/, '') + '/' + path.replace(/^\//, '');
    }
    return { proven, service, path };
  }
  const incomplete =
    input.candidateSets.some((set) => !set.complete) ||
    input.sourceSets.some((set) => !set.complete) ||
    !input.candidateSets.some((set) => set.selectorId === 'targets');
  if (incomplete) {
    for (const scope of config.scopes)
      gap(
        scope.id,
        '',
        'CANDIDATE_SET_INCOMPLETE',
        'Exported candidate/source selectors are missing or incomplete.',
      );
  } else {
    for (const request of input.entities.filter((entity) => entity.kind === 'request')) {
      const assertions = request.assertions.filter(original);
      const callRefs = edges('requests', null, request.entityId).flatMap(proofs);
      for (const assertion of assertions) {
        const attributes = assertion.attributes;
        if (!attributes.method || !attributes.path) {
          if (
            !edges('references', request.entityId).some(
              (edge) => entities.get(edge.toId)?.kind === 'graphqlOperation',
            )
          )
            gap(
              request.scopeId,
              request.entityId,
              'DYNAMIC_TARGET',
              'Request has no static HTTP target or collected GraphQL operation.',
            );
          continue;
        }
        const binding = requestBinding(request, attributes);
        const candidates = input.entities.filter(
          (entity) =>
            entity.kind === 'httpEndpoint' &&
            fields(entity).method === attributes.method &&
            fields(entity).registeredPath === binding.path,
        );
        const matching = candidates.filter(
          (candidate) =>
            fields(candidate).serviceId === binding.service?.id &&
            candidate.scopeId === binding.service.scopeId &&
            proofs(candidate).length &&
            edges('handles', candidate.entityId).length,
        );
        const confirmed =
          binding.proven &&
          callRefs.length > 0 &&
          matching.length === 1 &&
          assertions.every(
            (other) => JSON.stringify(other.attributes) === JSON.stringify(attributes),
          );
        const selected = binding.proven ? matching : candidates;
        if (!confirmed)
          gap(
            request.scopeId,
            request.entityId,
            selected.length > 1 ? 'AMBIGUOUS_BINDING' : 'BINDING_EVIDENCE_MISSING',
            'HTTP target is not uniquely supported by the configured client/service and original handler registration.',
            selected.map((item) => item.entityId),
          );
        for (const endpoint of selected)
          relation(
            'references',
            request.entityId,
            endpoint.entityId,
            [
              ...assertion.evidenceRefs,
              ...callRefs,
              ...proofs(endpoint),
              ...edges('handles', endpoint.entityId).flatMap(proofs),
            ],
            confirmed,
            humanProof([
              request,
              ...edges('requests', null, request.entityId),
              ...edges('handles', endpoint.entityId),
            ]),
          );
      }
    }
    for (const operation of input.entities.filter((entity) => entity.kind === 'graphqlOperation')) {
      const requests = edges('references', null, operation.entityId)
        .map((edge) => entities.get(edge.fromId))
        .filter((item) => item?.kind === 'request');
      const bindings = requests.flatMap((request) =>
        request.assertions.filter(original).map((assertion) => ({
          request,
          binding: requestBinding(request, assertion.attributes),
        })),
      );
      const owned =
        bindings.length > 0 &&
        bindings.every(
          ({ binding }) => binding.proven && binding.service?.id === fields(operation).serviceId,
        );
      for (const selection of edges('references', operation.entityId)) {
        const field = entities.get(selection.toId);
        if (field?.kind !== 'graphqlField') continue;
        const resolvers = edges('resolves', field.entityId).filter(
          (edge) => entities.get(edge.toId)?.scopeId === field.scopeId,
        );
        const targets = [...new Set(resolvers.map((edge) => edge.toId))];
        if (!owned || targets.length !== 1)
          gap(
            operation.scopeId,
            operation.entityId,
            targets.length > 1 ? 'AMBIGUOUS_BINDING' : 'BINDING_EVIDENCE_MISSING',
            'GraphQL schema field has no unique resolver with explicit client/service ownership.',
            targets,
          );
        for (const target of targets)
          relation(
            'calls',
            operation.entityId,
            target,
            [
              ...proofs(selection),
              ...resolvers.filter((edge) => edge.toId === target).flatMap(proofs),
              ...bindings.flatMap(({ request }) =>
                edges('requests', null, request.entityId).flatMap(proofs),
              ),
            ],
            owned && targets.length === 1,
            humanProof([selection, ...resolvers, ...requests]),
          );
      }
    }
    for (const query of input.entities.filter((entity) => entity.kind === 'queryImplementation')) {
      const reads = edges('reads', query.entityId).filter((edge) =>
        ['dataTable', 'dataColumn', 'dataKey'].includes(entities.get(edge.toId)?.kind),
      );
      const ids = [...new Set(reads.map((edge) => fields(entities.get(edge.toId)).dataSourceId))];
      const sources = input.entities.filter(
        (entity) =>
          entity.kind === 'dataSource' &&
          ids.includes(fields(entity).configuredId) &&
          entity.scopeId === query.scopeId,
      );
      if (!reads.length || sources.length !== 1 || ids.length !== 1)
        gap(
          query.scopeId,
          query.entityId,
          'BINDING_EVIDENCE_MISSING',
          'Query has no unique dataSource derived from proven table/column reads.',
          sources.map((entity) => entity.entityId),
        );
      for (const source of sources)
        relation(
          'reads',
          query.entityId,
          source.entityId,
          [...reads.flatMap(proofs), ...proofs(source)],
          sources.length === 1 && ids.length === 1,
          humanProof(reads),
        );
    }
  }
  output.relations = [...emittedRelations.values()];
  output.evidence = [...emittedEvidence.values()];
  for (const scope of config.scopes) {
    const gaps = output.gaps.filter((item) => item.scopeId === scope.id);
    output.coverage.push({
      capability: 'link',
      scopeId: scope.id,
      sourceSetId: input.sourceSets[0]?.selectorId ?? 'bindings',
      status: gaps.length ? 'partial' : output.relations.length ? 'complete' : 'notConfigured',
      counts: { relations: output.relations.length },
      gapIds: gaps.map((item) => item.id),
    });
  }
  return output;
}
