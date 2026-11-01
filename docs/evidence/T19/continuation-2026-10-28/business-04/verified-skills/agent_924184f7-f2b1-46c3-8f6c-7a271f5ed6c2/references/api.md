# api

Knowledge freshness: **fresh**. Each entry retains its own topic version and snapshot. Use the original references and Core status to check current facts.

```json
[
  {
    "topicId": "flow-guard",
    "freshness": "fresh",
    "snapshot": {
      "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
      "generation": "13",
      "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
    },
    "entities": [
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "13",
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
            "assertionId": "as_8b86417dd9604eba19ca67fb47684924118775c81c1ae8ba8e082a1fbd03f3d4",
            "producerKey": "core.base",
            "batchId": "bat_785f214ce29480d0f7c80c06e7f9175ef3986e0ed15c30a86353fd41a57d66e9",
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
        "bm25": -0.6171394698884836,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "13",
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
            "assertionId": "as_79ec66b08b93b1e02a0cce1e8430953cb765d0e7e97a271b7be5264cde61f109",
            "producerKey": "core.base",
            "batchId": "bat_785f214ce29480d0f7c80c06e7f9175ef3986e0ed15c30a86353fd41a57d66e9",
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
        "bm25": -0.5984805309280687,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "13",
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
            "assertionId": "as_9f9d32ff10e5c2c84bfb4072d9e83f0fc305bce25cf09c6a6c7c0d58b3252d5e",
            "producerKey": "core.base",
            "batchId": "bat_785f214ce29480d0f7c80c06e7f9175ef3986e0ed15c30a86353fd41a57d66e9",
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
        "bm25": -0.5487104752256264,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "13",
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
            "assertionId": "as_c92f093a4c08e7e9741d442e19d1db11228bd84ae296f51d96a3351fa0f1551d",
            "producerKey": "core.base",
            "batchId": "bat_785f214ce29480d0f7c80c06e7f9175ef3986e0ed15c30a86353fd41a57d66e9",
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
        "bm25": -0.5268057112136622,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "13",
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
            "assertionId": "as_16493fa7e42fa806d9ba874562b94884eec21d75cc427b5fae08a6301fd443d2",
            "producerKey": "core.base",
            "batchId": "bat_785f214ce29480d0f7c80c06e7f9175ef3986e0ed15c30a86353fd41a57d66e9",
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
        "bm25": -0.5065827063045951,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "13",
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
            "assertionId": "as_a245e197ef5fc7caa2ef5240b56734cf832ead92ea7fcc1cd570d1d48640a57a",
            "producerKey": "core.base",
            "batchId": "bat_785f214ce29480d0f7c80c06e7f9175ef3986e0ed15c30a86353fd41a57d66e9",
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
        "bm25": -0.48785494256111406,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "13",
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
            "assertionId": "as_bd32f13eab251e3506be52c353cfd115141199f9e2f3319696aafd864a2b8181",
            "producerKey": "core.base",
            "batchId": "bat_785f214ce29480d0f7c80c06e7f9175ef3986e0ed15c30a86353fd41a57d66e9",
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
        "bm25": -0.4704625001364696,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "13",
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
            "assertionId": "as_3e16178b46e5e4e087d577dd295362c5b7e3bc115cadd8624bdb206c1f096af1",
            "producerKey": "core.base",
            "batchId": "bat_785f214ce29480d0f7c80c06e7f9175ef3986e0ed15c30a86353fd41a57d66e9",
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
        "bm25": -0.45954049457510154,
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
      "generation": "13",
      "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
    },
    "entities": [
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "13",
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
            "assertionId": "as_8b86417dd9604eba19ca67fb47684924118775c81c1ae8ba8e082a1fbd03f3d4",
            "producerKey": "core.base",
            "batchId": "bat_785f214ce29480d0f7c80c06e7f9175ef3986e0ed15c30a86353fd41a57d66e9",
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
        "bm25": -0.6171394698884836,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "13",
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
            "assertionId": "as_79ec66b08b93b1e02a0cce1e8430953cb765d0e7e97a271b7be5264cde61f109",
            "producerKey": "core.base",
            "batchId": "bat_785f214ce29480d0f7c80c06e7f9175ef3986e0ed15c30a86353fd41a57d66e9",
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
        "bm25": -0.5984805309280687,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "13",
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
            "assertionId": "as_9f9d32ff10e5c2c84bfb4072d9e83f0fc305bce25cf09c6a6c7c0d58b3252d5e",
            "producerKey": "core.base",
            "batchId": "bat_785f214ce29480d0f7c80c06e7f9175ef3986e0ed15c30a86353fd41a57d66e9",
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
        "bm25": -0.5487104752256264,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "13",
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
            "assertionId": "as_c92f093a4c08e7e9741d442e19d1db11228bd84ae296f51d96a3351fa0f1551d",
            "producerKey": "core.base",
            "batchId": "bat_785f214ce29480d0f7c80c06e7f9175ef3986e0ed15c30a86353fd41a57d66e9",
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
        "bm25": -0.5268057112136622,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "13",
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
            "assertionId": "as_16493fa7e42fa806d9ba874562b94884eec21d75cc427b5fae08a6301fd443d2",
            "producerKey": "core.base",
            "batchId": "bat_785f214ce29480d0f7c80c06e7f9175ef3986e0ed15c30a86353fd41a57d66e9",
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
        "bm25": -0.5065827063045951,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "13",
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
            "assertionId": "as_a245e197ef5fc7caa2ef5240b56734cf832ead92ea7fcc1cd570d1d48640a57a",
            "producerKey": "core.base",
            "batchId": "bat_785f214ce29480d0f7c80c06e7f9175ef3986e0ed15c30a86353fd41a57d66e9",
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
        "bm25": -0.48785494256111406,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "13",
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
            "assertionId": "as_bd32f13eab251e3506be52c353cfd115141199f9e2f3319696aafd864a2b8181",
            "producerKey": "core.base",
            "batchId": "bat_785f214ce29480d0f7c80c06e7f9175ef3986e0ed15c30a86353fd41a57d66e9",
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
        "bm25": -0.4704625001364696,
        "projection": "entity"
      },
      {
        "entityRef": {
          "snapshot": {
            "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
            "generation": "13",
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
            "assertionId": "as_3e16178b46e5e4e087d577dd295362c5b7e3bc115cadd8624bdb206c1f096af1",
            "producerKey": "core.base",
            "batchId": "bat_785f214ce29480d0f7c80c06e7f9175ef3986e0ed15c30a86353fd41a57d66e9",
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
        "bm25": -0.45954049457510154,
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
