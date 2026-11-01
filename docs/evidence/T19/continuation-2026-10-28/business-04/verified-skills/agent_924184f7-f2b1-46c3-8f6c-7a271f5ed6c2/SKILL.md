---
name: purchase-workflow
description: Query the evidence-backed project knowledge and verify source freshness before use
---

# T19-audited-purchase project knowledge

Scope: the repositories and identities in `E:\Project\my-project\code-graph\docs\evidence\T19\continuation-2026-10-28\business-04\project\.project-knowledge\project.json`.
Rendered generation: 13. Knowledge freshness: **fresh**.

Use the installed `project-knowledge` CLI. First run:

```text
project-knowledge status --config "E:\Project\my-project\code-graph\docs\evidence\T19\continuation-2026-10-28\business-04\project\.project-knowledge\project.json" --check-freshness
project-knowledge query --config "E:\Project\my-project\code-graph\docs\evidence\T19\continuation-2026-10-28\business-04\project\.project-knowledge\project.json" --operation search --text "business term"
project-knowledge query --config "E:\Project\my-project\code-graph\docs\evidence\T19\continuation-2026-10-28\business-04\project\.project-knowledge\project.json" --request request.json
```

`request.json` uses the Core QueryRequest schema. Pass returned entity and source references unchanged. Read original source with `operation: readSource`; do not construct reference IDs. Inspect completeness, coverage, gaps and inferred claims before drawing conclusions. Stale explanations remain investigation clues until a new context is exported and a current topic is imported.

- [Conventions](references/conventions.md)
- [Routes](references/routes.md)
- [Frontend](references/frontend.md)
- [API](references/api.md)
- [Database](references/database.md)
- [Code map](references/code-map.md)
- [Original references](references/source-refs.md)
- [Version and file manifest](references/index-manifest.json)

## Topics and coverage gaps

- [flow-guard](references/topics/flow-guard.md) — version 2, fresh
- [purchase-workflow](references/topics/purchase-workflow.md) — version 10, fresh
