# code-map

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
    "data": {
      "files": [
        {
          "repositoryId": "business",
          "path": "flow.ts",
          "sourceId": "src_f4be1ceeaabcebaae684900161fc5eefbf24bd70e7f0d77a61cdd1bee2b64fac"
        }
      ],
      "symbols": [
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
          "name": "submit",
          "signature": "functionsubmit(order){audit(order);if(!(order.orderedQty>0&&order.supplierId))throwError('invalid order');order.state='SUBMITTED';save(order);}",
          "assertionId": "as_6dfc8ce5b791f2d6f1613337a303753c60344c2247e49d5f6e60927d5b7e6df9",
          "basis": "observed",
          "origin": "static",
          "rank": 0,
          "locations": [
            {
              "repositoryId": "business",
              "path": "flow.ts",
              "startByte": 16,
              "endByte": 22
            }
          ],
          "evidenceRefs": [
            "ev_85da46b3aa5388adf93f4d57957f07fdc9b31d76bc3ab6b9d697337d897f2aa4"
          ]
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
          "name": "withdraw",
          "signature": "functionwithdraw(order,actor){if(order.state==='SUBMITTED'&&order.applicant===actor.id)order.state='DRAFT';}",
          "assertionId": "as_4a744bfd54323bd557e015c5fe640ac3523b9c665d16648c7ff3236ac2028b87",
          "basis": "observed",
          "origin": "static",
          "rank": 0,
          "locations": [
            {
              "repositoryId": "business",
              "path": "flow.ts",
              "startByte": 708,
              "endByte": 716
            }
          ],
          "evidenceRefs": [
            "ev_f4f6d507a56b40c714a3a54599a66e42968555fcc8db9eaae3b285c7848c7837"
          ]
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
          "name": "finish",
          "signature": "functionfinish(order){if(order.netReceived>=order.orderedQty&&order.allReceiptsValid)order.state='COMPLETED';}",
          "assertionId": "as_837094f9fe39d63867445eb1c10c5da360fe2b6a2e0b93e42c292eaf380dba19",
          "basis": "observed",
          "origin": "static",
          "rank": 0,
          "locations": [
            {
              "repositoryId": "business",
              "path": "flow.ts",
              "startByte": 576,
              "endByte": 582
            }
          ],
          "evidenceRefs": [
            "ev_b7f9da54b83bf02a0d30fe92b8ca4749710420347f7d96c6f99bd831d98947f8"
          ]
        },
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
          "name": "#0",
          "signature": "(n,r)=>n+r.qty",
          "assertionId": "as_84a9f5616936d04dc5cf332e1a91e7e63f2a83ef9519d4f32ef8d9e0afa88162",
          "basis": "observed",
          "origin": "static",
          "rank": 0,
          "locations": [
            {
              "repositoryId": "business",
              "path": "flow.ts",
              "startByte": 502,
              "endByte": 521
            }
          ],
          "evidenceRefs": [
            "ev_0d311075baa662c1722c3d6f23ad25b93770c66304cec899348eab37eacacd2f"
          ]
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
          "name": "cancel",
          "signature": "functioncancel(order){if(order.state==='APPROVED'&&order.netReceived===0)order.state='CANCELLED';}",
          "assertionId": "as_65e62e503b3ad06914ea0668a70dd5088006331133f3edc5f6bcb78c21ed9acc",
          "basis": "observed",
          "origin": "static",
          "rank": 0,
          "locations": [
            {
              "repositoryId": "business",
              "path": "flow.ts",
              "startByte": 841,
              "endByte": 847
            }
          ],
          "evidenceRefs": [
            "ev_3a687092be02201e145cd570762ba87759341a5d740496e4cca267fe53000e70"
          ]
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
          "name": "#0",
          "signature": "r=>r.status==='POSTED'&&!r.deleted",
          "assertionId": "as_4a4a3fcfafa10f8f0a24fe9796c8faf9f4164fee0983a7a4e05b227b45e08dba",
          "basis": "observed",
          "origin": "static",
          "rank": 0,
          "locations": [
            {
              "repositoryId": "business",
              "path": "flow.ts",
              "startByte": 453,
              "endByte": 493
            }
          ],
          "evidenceRefs": [
            "ev_0030aa1aec5869e3658512290811193f8388bb7e40bd9342d51d36e95ce47c1f"
          ]
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
          "name": "receive",
          "signature": "functionreceive(order,receipts){if(receipts.filter(r=>r.status==='POSTED'&&!r.deleted).reduce((n,r)=>n+r.qty,0)>0)order.state='RECEIVING';}",
          "assertionId": "as_b219e0284e4c5a1d802272e29ac71d3e1f0eebdc556f995696fe46bfe096dd2a",
          "basis": "observed",
          "origin": "static",
          "rank": 0,
          "locations": [
            {
              "repositoryId": "business",
              "path": "flow.ts",
              "startByte": 404,
              "endByte": 411
            }
          ],
          "evidenceRefs": [
            "ev_0e6bef78a5b9e9c8a1d8d7a362e10a74d50b4fe5f0375f80f90af0c2e6da9769"
          ]
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
          "name": "approve",
          "signature": "functionapprove(order,actor){if(actor.role!=='buyer'||!actor.approvalGranted)throwError('not approved');order.state='APPROVED';}",
          "assertionId": "as_46d2d6aba16a74cd4f5551a1fc4ae0f3d2f11992677e75f40de285f178661f49",
          "basis": "observed",
          "origin": "static",
          "rank": 0,
          "locations": [
            {
              "repositoryId": "business",
              "path": "flow.ts",
              "startByte": 249,
              "endByte": 256
            }
          ],
          "evidenceRefs": [
            "ev_9886abe9de9f28216f3060bb4f42cf83db26e38370c7ee18864153dd97ba6a09"
          ]
        }
      ],
      "uncoveredLanguages": []
    }
  },
  {
    "topicId": "purchase-workflow",
    "freshness": "fresh",
    "snapshot": {
      "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
      "generation": "3",
      "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
    },
    "data": {
      "files": [
        {
          "repositoryId": "business",
          "path": "flow.ts",
          "sourceId": "src_f4be1ceeaabcebaae684900161fc5eefbf24bd70e7f0d77a61cdd1bee2b64fac"
        }
      ],
      "symbols": [
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
          "name": "submit",
          "signature": "functionsubmit(order){audit(order);if(!(order.orderedQty>0&&order.supplierId))throwError('invalid order');order.state='SUBMITTED';save(order);}",
          "assertionId": "as_6dfc8ce5b791f2d6f1613337a303753c60344c2247e49d5f6e60927d5b7e6df9",
          "basis": "observed",
          "origin": "static",
          "rank": 0,
          "locations": [
            {
              "repositoryId": "business",
              "path": "flow.ts",
              "startByte": 16,
              "endByte": 22
            }
          ],
          "evidenceRefs": [
            "ev_85da46b3aa5388adf93f4d57957f07fdc9b31d76bc3ab6b9d697337d897f2aa4"
          ]
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
          "name": "withdraw",
          "signature": "functionwithdraw(order,actor){if(order.state==='SUBMITTED'&&order.applicant===actor.id)order.state='DRAFT';}",
          "assertionId": "as_4a744bfd54323bd557e015c5fe640ac3523b9c665d16648c7ff3236ac2028b87",
          "basis": "observed",
          "origin": "static",
          "rank": 0,
          "locations": [
            {
              "repositoryId": "business",
              "path": "flow.ts",
              "startByte": 708,
              "endByte": 716
            }
          ],
          "evidenceRefs": [
            "ev_f4f6d507a56b40c714a3a54599a66e42968555fcc8db9eaae3b285c7848c7837"
          ]
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
          "name": "finish",
          "signature": "functionfinish(order){if(order.netReceived>=order.orderedQty&&order.allReceiptsValid)order.state='COMPLETED';}",
          "assertionId": "as_837094f9fe39d63867445eb1c10c5da360fe2b6a2e0b93e42c292eaf380dba19",
          "basis": "observed",
          "origin": "static",
          "rank": 0,
          "locations": [
            {
              "repositoryId": "business",
              "path": "flow.ts",
              "startByte": 576,
              "endByte": 582
            }
          ],
          "evidenceRefs": [
            "ev_b7f9da54b83bf02a0d30fe92b8ca4749710420347f7d96c6f99bd831d98947f8"
          ]
        },
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
          "name": "#0",
          "signature": "(n,r)=>n+r.qty",
          "assertionId": "as_84a9f5616936d04dc5cf332e1a91e7e63f2a83ef9519d4f32ef8d9e0afa88162",
          "basis": "observed",
          "origin": "static",
          "rank": 0,
          "locations": [
            {
              "repositoryId": "business",
              "path": "flow.ts",
              "startByte": 502,
              "endByte": 521
            }
          ],
          "evidenceRefs": [
            "ev_0d311075baa662c1722c3d6f23ad25b93770c66304cec899348eab37eacacd2f"
          ]
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
          "name": "cancel",
          "signature": "functioncancel(order){if(order.state==='APPROVED'&&order.netReceived===0)order.state='CANCELLED';}",
          "assertionId": "as_65e62e503b3ad06914ea0668a70dd5088006331133f3edc5f6bcb78c21ed9acc",
          "basis": "observed",
          "origin": "static",
          "rank": 0,
          "locations": [
            {
              "repositoryId": "business",
              "path": "flow.ts",
              "startByte": 841,
              "endByte": 847
            }
          ],
          "evidenceRefs": [
            "ev_3a687092be02201e145cd570762ba87759341a5d740496e4cca267fe53000e70"
          ]
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
          "name": "#0",
          "signature": "r=>r.status==='POSTED'&&!r.deleted",
          "assertionId": "as_4a4a3fcfafa10f8f0a24fe9796c8faf9f4164fee0983a7a4e05b227b45e08dba",
          "basis": "observed",
          "origin": "static",
          "rank": 0,
          "locations": [
            {
              "repositoryId": "business",
              "path": "flow.ts",
              "startByte": 453,
              "endByte": 493
            }
          ],
          "evidenceRefs": [
            "ev_0030aa1aec5869e3658512290811193f8388bb7e40bd9342d51d36e95ce47c1f"
          ]
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
          "name": "receive",
          "signature": "functionreceive(order,receipts){if(receipts.filter(r=>r.status==='POSTED'&&!r.deleted).reduce((n,r)=>n+r.qty,0)>0)order.state='RECEIVING';}",
          "assertionId": "as_b219e0284e4c5a1d802272e29ac71d3e1f0eebdc556f995696fe46bfe096dd2a",
          "basis": "observed",
          "origin": "static",
          "rank": 0,
          "locations": [
            {
              "repositoryId": "business",
              "path": "flow.ts",
              "startByte": 404,
              "endByte": 411
            }
          ],
          "evidenceRefs": [
            "ev_0e6bef78a5b9e9c8a1d8d7a362e10a74d50b4fe5f0375f80f90af0c2e6da9769"
          ]
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
          "name": "approve",
          "signature": "functionapprove(order,actor){if(actor.role!=='buyer'||!actor.approvalGranted)throwError('not approved');order.state='APPROVED';}",
          "assertionId": "as_46d2d6aba16a74cd4f5551a1fc4ae0f3d2f11992677e75f40de285f178661f49",
          "basis": "observed",
          "origin": "static",
          "rank": 0,
          "locations": [
            {
              "repositoryId": "business",
              "path": "flow.ts",
              "startByte": 249,
              "endByte": 256
            }
          ],
          "evidenceRefs": [
            "ev_9886abe9de9f28216f3060bb4f42cf83db26e38370c7ee18864153dd97ba6a09"
          ]
        }
      ],
      "uncoveredLanguages": []
    }
  }
]
```
