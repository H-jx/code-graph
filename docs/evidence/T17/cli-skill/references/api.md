# api

Knowledge freshness: **fresh**. Each entry retains its own topic version and snapshot. Use the original references and Core status to check current facts.

```json
[
  {
    "topicId": "ledger",
    "freshness": "fresh",
    "snapshot": {
      "indexId": "1c03c4cd-a108-44a3-abdc-64f98adae283",
      "generation": "1",
      "sourceSnapshotId": "snap_f4d908ac87c8fe748f929b77847b14f6522dd1edf7457b762160993e5fd03714"
    },
    "entities": [],
    "coverage": [
      {
        "replacementKey": "(core.base,main)",
        "capability": "api",
        "scopeId": "repo.main",
        "sourceSetId": "main",
        "status": "not_configured"
      },
      {
        "replacementKey": "(core.base,main)",
        "capability": "base",
        "scopeId": "repo.main",
        "sourceSetId": "main",
        "status": "complete"
      },
      {
        "replacementKey": "(core.base,main)",
        "capability": "database",
        "scopeId": "repo.main",
        "sourceSetId": "main",
        "status": "not_configured"
      },
      {
        "replacementKey": "(core.base,main)",
        "capability": "frontend",
        "scopeId": "repo.main",
        "sourceSetId": "main",
        "status": "not_configured"
      },
      {
        "replacementKey": "(core.base,main)",
        "capability": "report",
        "scopeId": "repo.main",
        "sourceSetId": "main",
        "status": "not_configured"
      },
      {
        "replacementKey": "(core.base,main)",
        "capability": "routes",
        "scopeId": "repo.main",
        "sourceSetId": "main",
        "status": "not_configured"
      },
      {
        "replacementKey": "(core.base,main)",
        "capability": "workflow",
        "scopeId": "repo.main",
        "sourceSetId": "main",
        "status": "not_configured"
      }
    ],
    "gaps": [
      {
        "id": "knowledge_material_partial",
        "code": "KNOWLEDGE_MATERIAL_PARTIAL",
        "capability": "knowledge",
        "scopeId": "",
        "sourceRefs": [],
        "candidateIds": [],
        "reason": "Some selected facts, outlines or source reads are partial; explanation completeness is limited by that material",
        "requiredEvidence": [
          "inspect completeness reasons and obtain the missing original material"
        ]
      }
    ]
  }
]
```
