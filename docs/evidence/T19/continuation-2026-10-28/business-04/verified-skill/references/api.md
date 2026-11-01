# api

Knowledge freshness: **fresh**. Each entry retains its own topic version and snapshot. Use the original references and Core status to check current facts.

```json
[
  {
    "topicId": "flow-guard",
    "freshness": "fresh",
    "snapshot": {
      "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
      "generation": "3",
      "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
    },
    "entities": [
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "3",
            "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
          },
          "entityId": "ent_94aa8fcc01c94d6ab408461bda147a6aa3b32ff23a18d91fd9bd37130f4977ef"
        },
        "kind": "symbol",
        "scopeId": "repo.business",
        "identity": [
          {
            "name": "repositoryId",
            "value": "business"
          },
          {
            "name": "path",
            "value": "flow.ts"
          },
          {
            "name": "scopeId",
            "value": "repo.business"
          },
          {
            "name": "lexicalPath",
            "value": "#0"
          },
          {
            "name": "symbolKind",
            "value": "function"
          },
          {
            "name": "normalizedSignature",
            "value": "(n,r)=>n+r.qty"
          }
        ],
        "name": "#0",
        "assertions": [
          {
            "assertionId": "as_84a9f5616936d04dc5cf332e1a91e7e63f2a83ef9519d4f32ef8d9e0afa88162",
            "producerKey": "core.base",
            "batchId": "bat_65303fb019695ebaa2608d034264609e8bc418190fb571ee0acdf0cfab45fa98",
            "basis": "observed",
            "origin": "static",
            "name": "#0",
            "attributes": {
              "name": "#0",
              "symbolKind": "function",
              "lexicalPath": "#0",
              "normalizedSignature": "(n,r)=>n+r.qty",
              "path": "flow.ts"
            },
            "evidenceRefs": [
              "ev_0d311075baa662c1722c3d6f23ad25b93770c66304cec899348eab37eacacd2f"
            ]
          }
        ],
        "matchReason": "identifier",
        "exactRank": 3,
        "bm25": -1.4975317828058217,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "3",
            "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
          },
          "entityId": "ent_b0b5b064fd114649ce14b6093c1fc22fddbb4a750d99b03eae2deffe6dc18542"
        },
        "kind": "symbol",
        "scopeId": "repo.business",
        "identity": [
          {
            "name": "repositoryId",
            "value": "business"
          },
          {
            "name": "path",
            "value": "flow.ts"
          },
          {
            "name": "scopeId",
            "value": "repo.business"
          },
          {
            "name": "lexicalPath",
            "value": "#0"
          },
          {
            "name": "symbolKind",
            "value": "function"
          },
          {
            "name": "normalizedSignature",
            "value": "r=>r.status==='POSTED'&&!r.deleted"
          }
        ],
        "name": "#0",
        "assertions": [
          {
            "assertionId": "as_4a4a3fcfafa10f8f0a24fe9796c8faf9f4164fee0983a7a4e05b227b45e08dba",
            "producerKey": "core.base",
            "batchId": "bat_65303fb019695ebaa2608d034264609e8bc418190fb571ee0acdf0cfab45fa98",
            "basis": "observed",
            "origin": "static",
            "name": "#0",
            "attributes": {
              "name": "#0",
              "symbolKind": "function",
              "lexicalPath": "#0",
              "normalizedSignature": "r=>r.status==='POSTED'&&!r.deleted",
              "path": "flow.ts"
            },
            "evidenceRefs": [
              "ev_0030aa1aec5869e3658512290811193f8388bb7e40bd9342d51d36e95ce47c1f"
            ]
          }
        ],
        "matchReason": "identifier",
        "exactRank": 3,
        "bm25": -1.4490536519988246,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "3",
            "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
          },
          "entityId": "ent_2be09b02c0e0a481fba690df7e88f43e1da5339165e757a9769561a927d232dd"
        },
        "kind": "symbol",
        "scopeId": "repo.business",
        "identity": [
          {
            "name": "repositoryId",
            "value": "business"
          },
          {
            "name": "path",
            "value": "flow.ts"
          },
          {
            "name": "scopeId",
            "value": "repo.business"
          },
          {
            "name": "lexicalPath",
            "value": "withdraw"
          },
          {
            "name": "symbolKind",
            "value": "function"
          },
          {
            "name": "normalizedSignature",
            "value": "functionwithdraw(order,actor){if(order.state==='SUBMITTED'&&order.applicant===actor.id)order.state='DRAFT';}"
          }
        ],
        "name": "withdraw",
        "assertions": [
          {
            "assertionId": "as_4a744bfd54323bd557e015c5fe640ac3523b9c665d16648c7ff3236ac2028b87",
            "producerKey": "core.base",
            "batchId": "bat_65303fb019695ebaa2608d034264609e8bc418190fb571ee0acdf0cfab45fa98",
            "basis": "observed",
            "origin": "static",
            "name": "withdraw",
            "attributes": {
              "name": "withdraw",
              "symbolKind": "function",
              "lexicalPath": "withdraw",
              "normalizedSignature": "functionwithdraw(order,actor){if(order.state==='SUBMITTED'&&order.applicant===actor.id)order.state='DRAFT';}",
              "path": "flow.ts"
            },
            "evidenceRefs": [
              "ev_f4f6d507a56b40c714a3a54599a66e42968555fcc8db9eaae3b285c7848c7837"
            ]
          }
        ],
        "matchReason": "identifier",
        "exactRank": 3,
        "bm25": -1.3207842812463968,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "3",
            "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
          },
          "entityId": "ent_ac51da2edd6599a32bf0165f48d3ee23f2ed4e8535678cda26c265055af6f65c"
        },
        "kind": "symbol",
        "scopeId": "repo.business",
        "identity": [
          {
            "name": "repositoryId",
            "value": "business"
          },
          {
            "name": "path",
            "value": "flow.ts"
          },
          {
            "name": "scopeId",
            "value": "repo.business"
          },
          {
            "name": "lexicalPath",
            "value": "cancel"
          },
          {
            "name": "symbolKind",
            "value": "function"
          },
          {
            "name": "normalizedSignature",
            "value": "functioncancel(order){if(order.state==='APPROVED'&&order.netReceived===0)order.state='CANCELLED';}"
          }
        ],
        "name": "cancel",
        "assertions": [
          {
            "assertionId": "as_65e62e503b3ad06914ea0668a70dd5088006331133f3edc5f6bcb78c21ed9acc",
            "producerKey": "core.base",
            "batchId": "bat_65303fb019695ebaa2608d034264609e8bc418190fb571ee0acdf0cfab45fa98",
            "basis": "observed",
            "origin": "static",
            "name": "cancel",
            "attributes": {
              "name": "cancel",
              "symbolKind": "function",
              "lexicalPath": "cancel",
              "normalizedSignature": "functioncancel(order){if(order.state==='APPROVED'&&order.netReceived===0)order.state='CANCELLED';}",
              "path": "flow.ts"
            },
            "evidenceRefs": [
              "ev_3a687092be02201e145cd570762ba87759341a5d740496e4cca267fe53000e70"
            ]
          }
        ],
        "matchReason": "identifier",
        "exactRank": 3,
        "bm25": -1.2648044126237428,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "3",
            "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
          },
          "entityId": "ent_eb8eade2991bc10ebe1f6c5c1cd01fb72c238db828046321c24fba5100ba1363"
        },
        "kind": "symbol",
        "scopeId": "repo.business",
        "identity": [
          {
            "name": "repositoryId",
            "value": "business"
          },
          {
            "name": "path",
            "value": "flow.ts"
          },
          {
            "name": "scopeId",
            "value": "repo.business"
          },
          {
            "name": "lexicalPath",
            "value": "receive"
          },
          {
            "name": "symbolKind",
            "value": "function"
          },
          {
            "name": "normalizedSignature",
            "value": "functionreceive(order,receipts){if(receipts.filter(r=>r.status==='POSTED'&&!r.deleted).reduce((n,r)=>n+r.qty,0)>0)order.state='RECEIVING';}"
          }
        ],
        "name": "receive",
        "assertions": [
          {
            "assertionId": "as_b219e0284e4c5a1d802272e29ac71d3e1f0eebdc556f995696fe46bfe096dd2a",
            "producerKey": "core.base",
            "batchId": "bat_65303fb019695ebaa2608d034264609e8bc418190fb571ee0acdf0cfab45fa98",
            "basis": "observed",
            "origin": "static",
            "name": "receive",
            "attributes": {
              "name": "receive",
              "symbolKind": "function",
              "lexicalPath": "receive",
              "normalizedSignature": "functionreceive(order,receipts){if(receipts.filter(r=>r.status==='POSTED'&&!r.deleted).reduce((n,r)=>n+r.qty,0)>0)order.state='RECEIVING';}",
              "path": "flow.ts"
            },
            "evidenceRefs": [
              "ev_0e6bef78a5b9e9c8a1d8d7a362e10a74d50b4fe5f0375f80f90af0c2e6da9769"
            ]
          }
        ],
        "matchReason": "identifier",
        "exactRank": 3,
        "bm25": -1.213376878823735,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "3",
            "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
          },
          "entityId": "ent_ffedfccfaba5706caab8f220e93406d246bac63cb6b8ffdda79bcb05c505729a"
        },
        "kind": "symbol",
        "scopeId": "repo.business",
        "identity": [
          {
            "name": "repositoryId",
            "value": "business"
          },
          {
            "name": "path",
            "value": "flow.ts"
          },
          {
            "name": "scopeId",
            "value": "repo.business"
          },
          {
            "name": "lexicalPath",
            "value": "approve"
          },
          {
            "name": "symbolKind",
            "value": "function"
          },
          {
            "name": "normalizedSignature",
            "value": "functionapprove(order,actor){if(actor.role!=='buyer'||!actor.approvalGranted)throwError('not approved');order.state='APPROVED';}"
          }
        ],
        "name": "approve",
        "assertions": [
          {
            "assertionId": "as_46d2d6aba16a74cd4f5551a1fc4ae0f3d2f11992677e75f40de285f178661f49",
            "producerKey": "core.base",
            "batchId": "bat_65303fb019695ebaa2608d034264609e8bc418190fb571ee0acdf0cfab45fa98",
            "basis": "observed",
            "origin": "static",
            "name": "approve",
            "attributes": {
              "name": "approve",
              "symbolKind": "function",
              "lexicalPath": "approve",
              "normalizedSignature": "functionapprove(order,actor){if(actor.role!=='buyer'||!actor.approvalGranted)throwError('not approved');order.state='APPROVED';}",
              "path": "flow.ts"
            },
            "evidenceRefs": [
              "ev_9886abe9de9f28216f3060bb4f42cf83db26e38370c7ee18864153dd97ba6a09"
            ]
          }
        ],
        "matchReason": "identifier",
        "exactRank": 3,
        "bm25": -1.165968076281446,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "3",
            "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
          },
          "entityId": "ent_69dd1e3d77c58c2aeb8b43284afe3f8c99ec28ae356b57bd68ac118b450b0d84"
        },
        "kind": "symbol",
        "scopeId": "repo.business",
        "identity": [
          {
            "name": "repositoryId",
            "value": "business"
          },
          {
            "name": "path",
            "value": "flow.ts"
          },
          {
            "name": "scopeId",
            "value": "repo.business"
          },
          {
            "name": "lexicalPath",
            "value": "finish"
          },
          {
            "name": "symbolKind",
            "value": "function"
          },
          {
            "name": "normalizedSignature",
            "value": "functionfinish(order){if(order.netReceived>=order.orderedQty&&order.allReceiptsValid)order.state='COMPLETED';}"
          }
        ],
        "name": "finish",
        "assertions": [
          {
            "assertionId": "as_837094f9fe39d63867445eb1c10c5da360fe2b6a2e0b93e42c292eaf380dba19",
            "producerKey": "core.base",
            "batchId": "bat_65303fb019695ebaa2608d034264609e8bc418190fb571ee0acdf0cfab45fa98",
            "basis": "observed",
            "origin": "static",
            "name": "finish",
            "attributes": {
              "name": "finish",
              "symbolKind": "function",
              "lexicalPath": "finish",
              "normalizedSignature": "functionfinish(order){if(order.netReceived>=order.orderedQty&&order.allReceiptsValid)order.state='COMPLETED';}",
              "path": "flow.ts"
            },
            "evidenceRefs": [
              "ev_b7f9da54b83bf02a0d30fe92b8ca4749710420347f7d96c6f99bd831d98947f8"
            ]
          }
        ],
        "matchReason": "identifier",
        "exactRank": 3,
        "bm25": -1.1221246609286097,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "3",
            "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
          },
          "entityId": "ent_0635e7568e888ce2c0afb1ed92a4896ca85877dc724e60a282c654527547262a"
        },
        "kind": "symbol",
        "scopeId": "repo.business",
        "identity": [
          {
            "name": "repositoryId",
            "value": "business"
          },
          {
            "name": "path",
            "value": "flow.ts"
          },
          {
            "name": "scopeId",
            "value": "repo.business"
          },
          {
            "name": "lexicalPath",
            "value": "submit"
          },
          {
            "name": "symbolKind",
            "value": "function"
          },
          {
            "name": "normalizedSignature",
            "value": "functionsubmit(order){audit(order);if(!(order.orderedQty>0&&order.supplierId))throwError('invalid order');order.state='SUBMITTED';save(order);}"
          }
        ],
        "name": "submit",
        "assertions": [
          {
            "assertionId": "as_6dfc8ce5b791f2d6f1613337a303753c60344c2247e49d5f6e60927d5b7e6df9",
            "producerKey": "core.base",
            "batchId": "bat_65303fb019695ebaa2608d034264609e8bc418190fb571ee0acdf0cfab45fa98",
            "basis": "observed",
            "origin": "static",
            "name": "submit",
            "attributes": {
              "name": "submit",
              "symbolKind": "function",
              "lexicalPath": "submit",
              "normalizedSignature": "functionsubmit(order){audit(order);if(!(order.orderedQty>0&&order.supplierId))throwError('invalid order');order.state='SUBMITTED';save(order);}",
              "path": "flow.ts"
            },
            "evidenceRefs": [
              "ev_85da46b3aa5388adf93f4d57957f07fdc9b31d76bc3ab6b9d697337d897f2aa4"
            ]
          }
        ],
        "matchReason": "identifier",
        "exactRank": 3,
        "bm25": -1.094682726380208,
        "projection": "entity"
      }
    ],
    "coverage": [
      {
        "replacementKey": "(core.base,business)",
        "capability": "api",
        "scopeId": "repo.business",
        "sourceSetId": "business",
        "status": "not_configured"
      },
      {
        "replacementKey": "(core.base,business)",
        "capability": "base",
        "scopeId": "repo.business",
        "sourceSetId": "business",
        "status": "complete"
      },
      {
        "replacementKey": "(core.base,business)",
        "capability": "database",
        "scopeId": "repo.business",
        "sourceSetId": "business",
        "status": "not_configured"
      },
      {
        "replacementKey": "(core.base,business)",
        "capability": "frontend",
        "scopeId": "repo.business",
        "sourceSetId": "business",
        "status": "not_configured"
      },
      {
        "replacementKey": "(core.base,business)",
        "capability": "report",
        "scopeId": "repo.business",
        "sourceSetId": "business",
        "status": "not_configured"
      },
      {
        "replacementKey": "(core.base,business)",
        "capability": "routes",
        "scopeId": "repo.business",
        "sourceSetId": "business",
        "status": "not_configured"
      },
      {
        "replacementKey": "(core.base,business)",
        "capability": "workflow",
        "scopeId": "repo.business",
        "sourceSetId": "business",
        "status": "not_configured"
      },
      {
        "replacementKey": "(t19-audited.workflow,extract,T19-audited-purchase)",
        "capability": "workflow",
        "scopeId": "repo.business",
        "sourceSetId": "rules",
        "status": "complete"
      }
    ],
    "gaps": []
  },
  {
    "topicId": "purchase-workflow",
    "freshness": "fresh",
    "snapshot": {
      "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
      "generation": "3",
      "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
    },
    "entities": [
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "3",
            "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
          },
          "entityId": "ent_94aa8fcc01c94d6ab408461bda147a6aa3b32ff23a18d91fd9bd37130f4977ef"
        },
        "kind": "symbol",
        "scopeId": "repo.business",
        "identity": [
          {
            "name": "repositoryId",
            "value": "business"
          },
          {
            "name": "path",
            "value": "flow.ts"
          },
          {
            "name": "scopeId",
            "value": "repo.business"
          },
          {
            "name": "lexicalPath",
            "value": "#0"
          },
          {
            "name": "symbolKind",
            "value": "function"
          },
          {
            "name": "normalizedSignature",
            "value": "(n,r)=>n+r.qty"
          }
        ],
        "name": "#0",
        "assertions": [
          {
            "assertionId": "as_84a9f5616936d04dc5cf332e1a91e7e63f2a83ef9519d4f32ef8d9e0afa88162",
            "producerKey": "core.base",
            "batchId": "bat_65303fb019695ebaa2608d034264609e8bc418190fb571ee0acdf0cfab45fa98",
            "basis": "observed",
            "origin": "static",
            "name": "#0",
            "attributes": {
              "name": "#0",
              "symbolKind": "function",
              "lexicalPath": "#0",
              "normalizedSignature": "(n,r)=>n+r.qty",
              "path": "flow.ts"
            },
            "evidenceRefs": [
              "ev_0d311075baa662c1722c3d6f23ad25b93770c66304cec899348eab37eacacd2f"
            ]
          }
        ],
        "matchReason": "identifier",
        "exactRank": 3,
        "bm25": -1.4975317828058217,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "3",
            "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
          },
          "entityId": "ent_b0b5b064fd114649ce14b6093c1fc22fddbb4a750d99b03eae2deffe6dc18542"
        },
        "kind": "symbol",
        "scopeId": "repo.business",
        "identity": [
          {
            "name": "repositoryId",
            "value": "business"
          },
          {
            "name": "path",
            "value": "flow.ts"
          },
          {
            "name": "scopeId",
            "value": "repo.business"
          },
          {
            "name": "lexicalPath",
            "value": "#0"
          },
          {
            "name": "symbolKind",
            "value": "function"
          },
          {
            "name": "normalizedSignature",
            "value": "r=>r.status==='POSTED'&&!r.deleted"
          }
        ],
        "name": "#0",
        "assertions": [
          {
            "assertionId": "as_4a4a3fcfafa10f8f0a24fe9796c8faf9f4164fee0983a7a4e05b227b45e08dba",
            "producerKey": "core.base",
            "batchId": "bat_65303fb019695ebaa2608d034264609e8bc418190fb571ee0acdf0cfab45fa98",
            "basis": "observed",
            "origin": "static",
            "name": "#0",
            "attributes": {
              "name": "#0",
              "symbolKind": "function",
              "lexicalPath": "#0",
              "normalizedSignature": "r=>r.status==='POSTED'&&!r.deleted",
              "path": "flow.ts"
            },
            "evidenceRefs": [
              "ev_0030aa1aec5869e3658512290811193f8388bb7e40bd9342d51d36e95ce47c1f"
            ]
          }
        ],
        "matchReason": "identifier",
        "exactRank": 3,
        "bm25": -1.4490536519988246,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "3",
            "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
          },
          "entityId": "ent_2be09b02c0e0a481fba690df7e88f43e1da5339165e757a9769561a927d232dd"
        },
        "kind": "symbol",
        "scopeId": "repo.business",
        "identity": [
          {
            "name": "repositoryId",
            "value": "business"
          },
          {
            "name": "path",
            "value": "flow.ts"
          },
          {
            "name": "scopeId",
            "value": "repo.business"
          },
          {
            "name": "lexicalPath",
            "value": "withdraw"
          },
          {
            "name": "symbolKind",
            "value": "function"
          },
          {
            "name": "normalizedSignature",
            "value": "functionwithdraw(order,actor){if(order.state==='SUBMITTED'&&order.applicant===actor.id)order.state='DRAFT';}"
          }
        ],
        "name": "withdraw",
        "assertions": [
          {
            "assertionId": "as_4a744bfd54323bd557e015c5fe640ac3523b9c665d16648c7ff3236ac2028b87",
            "producerKey": "core.base",
            "batchId": "bat_65303fb019695ebaa2608d034264609e8bc418190fb571ee0acdf0cfab45fa98",
            "basis": "observed",
            "origin": "static",
            "name": "withdraw",
            "attributes": {
              "name": "withdraw",
              "symbolKind": "function",
              "lexicalPath": "withdraw",
              "normalizedSignature": "functionwithdraw(order,actor){if(order.state==='SUBMITTED'&&order.applicant===actor.id)order.state='DRAFT';}",
              "path": "flow.ts"
            },
            "evidenceRefs": [
              "ev_f4f6d507a56b40c714a3a54599a66e42968555fcc8db9eaae3b285c7848c7837"
            ]
          }
        ],
        "matchReason": "identifier",
        "exactRank": 3,
        "bm25": -1.3207842812463968,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "3",
            "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
          },
          "entityId": "ent_ac51da2edd6599a32bf0165f48d3ee23f2ed4e8535678cda26c265055af6f65c"
        },
        "kind": "symbol",
        "scopeId": "repo.business",
        "identity": [
          {
            "name": "repositoryId",
            "value": "business"
          },
          {
            "name": "path",
            "value": "flow.ts"
          },
          {
            "name": "scopeId",
            "value": "repo.business"
          },
          {
            "name": "lexicalPath",
            "value": "cancel"
          },
          {
            "name": "symbolKind",
            "value": "function"
          },
          {
            "name": "normalizedSignature",
            "value": "functioncancel(order){if(order.state==='APPROVED'&&order.netReceived===0)order.state='CANCELLED';}"
          }
        ],
        "name": "cancel",
        "assertions": [
          {
            "assertionId": "as_65e62e503b3ad06914ea0668a70dd5088006331133f3edc5f6bcb78c21ed9acc",
            "producerKey": "core.base",
            "batchId": "bat_65303fb019695ebaa2608d034264609e8bc418190fb571ee0acdf0cfab45fa98",
            "basis": "observed",
            "origin": "static",
            "name": "cancel",
            "attributes": {
              "name": "cancel",
              "symbolKind": "function",
              "lexicalPath": "cancel",
              "normalizedSignature": "functioncancel(order){if(order.state==='APPROVED'&&order.netReceived===0)order.state='CANCELLED';}",
              "path": "flow.ts"
            },
            "evidenceRefs": [
              "ev_3a687092be02201e145cd570762ba87759341a5d740496e4cca267fe53000e70"
            ]
          }
        ],
        "matchReason": "identifier",
        "exactRank": 3,
        "bm25": -1.2648044126237428,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "3",
            "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
          },
          "entityId": "ent_eb8eade2991bc10ebe1f6c5c1cd01fb72c238db828046321c24fba5100ba1363"
        },
        "kind": "symbol",
        "scopeId": "repo.business",
        "identity": [
          {
            "name": "repositoryId",
            "value": "business"
          },
          {
            "name": "path",
            "value": "flow.ts"
          },
          {
            "name": "scopeId",
            "value": "repo.business"
          },
          {
            "name": "lexicalPath",
            "value": "receive"
          },
          {
            "name": "symbolKind",
            "value": "function"
          },
          {
            "name": "normalizedSignature",
            "value": "functionreceive(order,receipts){if(receipts.filter(r=>r.status==='POSTED'&&!r.deleted).reduce((n,r)=>n+r.qty,0)>0)order.state='RECEIVING';}"
          }
        ],
        "name": "receive",
        "assertions": [
          {
            "assertionId": "as_b219e0284e4c5a1d802272e29ac71d3e1f0eebdc556f995696fe46bfe096dd2a",
            "producerKey": "core.base",
            "batchId": "bat_65303fb019695ebaa2608d034264609e8bc418190fb571ee0acdf0cfab45fa98",
            "basis": "observed",
            "origin": "static",
            "name": "receive",
            "attributes": {
              "name": "receive",
              "symbolKind": "function",
              "lexicalPath": "receive",
              "normalizedSignature": "functionreceive(order,receipts){if(receipts.filter(r=>r.status==='POSTED'&&!r.deleted).reduce((n,r)=>n+r.qty,0)>0)order.state='RECEIVING';}",
              "path": "flow.ts"
            },
            "evidenceRefs": [
              "ev_0e6bef78a5b9e9c8a1d8d7a362e10a74d50b4fe5f0375f80f90af0c2e6da9769"
            ]
          }
        ],
        "matchReason": "identifier",
        "exactRank": 3,
        "bm25": -1.213376878823735,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "3",
            "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
          },
          "entityId": "ent_ffedfccfaba5706caab8f220e93406d246bac63cb6b8ffdda79bcb05c505729a"
        },
        "kind": "symbol",
        "scopeId": "repo.business",
        "identity": [
          {
            "name": "repositoryId",
            "value": "business"
          },
          {
            "name": "path",
            "value": "flow.ts"
          },
          {
            "name": "scopeId",
            "value": "repo.business"
          },
          {
            "name": "lexicalPath",
            "value": "approve"
          },
          {
            "name": "symbolKind",
            "value": "function"
          },
          {
            "name": "normalizedSignature",
            "value": "functionapprove(order,actor){if(actor.role!=='buyer'||!actor.approvalGranted)throwError('not approved');order.state='APPROVED';}"
          }
        ],
        "name": "approve",
        "assertions": [
          {
            "assertionId": "as_46d2d6aba16a74cd4f5551a1fc4ae0f3d2f11992677e75f40de285f178661f49",
            "producerKey": "core.base",
            "batchId": "bat_65303fb019695ebaa2608d034264609e8bc418190fb571ee0acdf0cfab45fa98",
            "basis": "observed",
            "origin": "static",
            "name": "approve",
            "attributes": {
              "name": "approve",
              "symbolKind": "function",
              "lexicalPath": "approve",
              "normalizedSignature": "functionapprove(order,actor){if(actor.role!=='buyer'||!actor.approvalGranted)throwError('not approved');order.state='APPROVED';}",
              "path": "flow.ts"
            },
            "evidenceRefs": [
              "ev_9886abe9de9f28216f3060bb4f42cf83db26e38370c7ee18864153dd97ba6a09"
            ]
          }
        ],
        "matchReason": "identifier",
        "exactRank": 3,
        "bm25": -1.165968076281446,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "3",
            "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
          },
          "entityId": "ent_69dd1e3d77c58c2aeb8b43284afe3f8c99ec28ae356b57bd68ac118b450b0d84"
        },
        "kind": "symbol",
        "scopeId": "repo.business",
        "identity": [
          {
            "name": "repositoryId",
            "value": "business"
          },
          {
            "name": "path",
            "value": "flow.ts"
          },
          {
            "name": "scopeId",
            "value": "repo.business"
          },
          {
            "name": "lexicalPath",
            "value": "finish"
          },
          {
            "name": "symbolKind",
            "value": "function"
          },
          {
            "name": "normalizedSignature",
            "value": "functionfinish(order){if(order.netReceived>=order.orderedQty&&order.allReceiptsValid)order.state='COMPLETED';}"
          }
        ],
        "name": "finish",
        "assertions": [
          {
            "assertionId": "as_837094f9fe39d63867445eb1c10c5da360fe2b6a2e0b93e42c292eaf380dba19",
            "producerKey": "core.base",
            "batchId": "bat_65303fb019695ebaa2608d034264609e8bc418190fb571ee0acdf0cfab45fa98",
            "basis": "observed",
            "origin": "static",
            "name": "finish",
            "attributes": {
              "name": "finish",
              "symbolKind": "function",
              "lexicalPath": "finish",
              "normalizedSignature": "functionfinish(order){if(order.netReceived>=order.orderedQty&&order.allReceiptsValid)order.state='COMPLETED';}",
              "path": "flow.ts"
            },
            "evidenceRefs": [
              "ev_b7f9da54b83bf02a0d30fe92b8ca4749710420347f7d96c6f99bd831d98947f8"
            ]
          }
        ],
        "matchReason": "identifier",
        "exactRank": 3,
        "bm25": -1.1221246609286097,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "3",
            "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
          },
          "entityId": "ent_0635e7568e888ce2c0afb1ed92a4896ca85877dc724e60a282c654527547262a"
        },
        "kind": "symbol",
        "scopeId": "repo.business",
        "identity": [
          {
            "name": "repositoryId",
            "value": "business"
          },
          {
            "name": "path",
            "value": "flow.ts"
          },
          {
            "name": "scopeId",
            "value": "repo.business"
          },
          {
            "name": "lexicalPath",
            "value": "submit"
          },
          {
            "name": "symbolKind",
            "value": "function"
          },
          {
            "name": "normalizedSignature",
            "value": "functionsubmit(order){audit(order);if(!(order.orderedQty>0&&order.supplierId))throwError('invalid order');order.state='SUBMITTED';save(order);}"
          }
        ],
        "name": "submit",
        "assertions": [
          {
            "assertionId": "as_6dfc8ce5b791f2d6f1613337a303753c60344c2247e49d5f6e60927d5b7e6df9",
            "producerKey": "core.base",
            "batchId": "bat_65303fb019695ebaa2608d034264609e8bc418190fb571ee0acdf0cfab45fa98",
            "basis": "observed",
            "origin": "static",
            "name": "submit",
            "attributes": {
              "name": "submit",
              "symbolKind": "function",
              "lexicalPath": "submit",
              "normalizedSignature": "functionsubmit(order){audit(order);if(!(order.orderedQty>0&&order.supplierId))throwError('invalid order');order.state='SUBMITTED';save(order);}",
              "path": "flow.ts"
            },
            "evidenceRefs": [
              "ev_85da46b3aa5388adf93f4d57957f07fdc9b31d76bc3ab6b9d697337d897f2aa4"
            ]
          }
        ],
        "matchReason": "identifier",
        "exactRank": 3,
        "bm25": -1.094682726380208,
        "projection": "entity"
      }
    ],
    "coverage": [
      {
        "replacementKey": "(core.base,business)",
        "capability": "api",
        "scopeId": "repo.business",
        "sourceSetId": "business",
        "status": "not_configured"
      },
      {
        "replacementKey": "(core.base,business)",
        "capability": "base",
        "scopeId": "repo.business",
        "sourceSetId": "business",
        "status": "complete"
      },
      {
        "replacementKey": "(core.base,business)",
        "capability": "database",
        "scopeId": "repo.business",
        "sourceSetId": "business",
        "status": "not_configured"
      },
      {
        "replacementKey": "(core.base,business)",
        "capability": "frontend",
        "scopeId": "repo.business",
        "sourceSetId": "business",
        "status": "not_configured"
      },
      {
        "replacementKey": "(core.base,business)",
        "capability": "report",
        "scopeId": "repo.business",
        "sourceSetId": "business",
        "status": "not_configured"
      },
      {
        "replacementKey": "(core.base,business)",
        "capability": "routes",
        "scopeId": "repo.business",
        "sourceSetId": "business",
        "status": "not_configured"
      },
      {
        "replacementKey": "(core.base,business)",
        "capability": "workflow",
        "scopeId": "repo.business",
        "sourceSetId": "business",
        "status": "not_configured"
      },
      {
        "replacementKey": "(t19-audited.workflow,extract,T19-audited-purchase)",
        "capability": "workflow",
        "scopeId": "repo.business",
        "sourceSetId": "rules",
        "status": "complete"
      }
    ],
    "gaps": []
  }
]
```
