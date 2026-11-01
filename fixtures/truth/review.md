# T06 source review — 2026-10-28 Asia/Shanghai

Reviewer: Codex, source inspection performed before implementing the business collectors.
Method: read original registration, i18n, page, GraphQL, state/workflow, ORM and permissions materials;
compare independent files and the original pre-existing tests. No collector output was used.
Original authors/revisions per file are preserved in fixtures/original/provenance.json.
This record does not claim human sign-off or that original authors reviewed this truth package.

Scope: scenario1/2 original facts are source-declared implementation/document facts, not an assertion
that business requirements are correct. The route's actual menu is 生产采购申请. The historical e2e
permission comment differs from current routes.ts; both are retained as a conflict, not silently merged.
GraphQL shared transport does not prove any other field/resolver relationship. Unknown database instance
bindings and numeric production rules remain gaps. The synthetic DRAFT/etc vocabulary is separate.

The A/B TS/TSX and JS/TSX+Python fixtures are explicitly smoke models. Their static source was read
against frozen frontend expectations. No real-project accuracy percentage is assigned to them.
The arithmetic sample has a separate audit: integer-cent calculation from individual input records,
compared with in-memory SQLite preaggregation. Fanout and WHERE-child mutants must disagree.
The audited arithmetic validates only the stated acceptance policy, never production accounting.

Truth change policy: immutable baseline. Any change requires a new review reason and old/new truthHash;
tests must fail on changed source/hash, missing citations, changed expected or weakened negative cases.
Candidate tools write only owned work/agent run revisions; they cannot choose a truth output path.
