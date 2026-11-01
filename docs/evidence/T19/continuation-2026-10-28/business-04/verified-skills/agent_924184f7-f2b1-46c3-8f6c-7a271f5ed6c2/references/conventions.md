# Original evidence and derived conventions

Knowledge freshness: **fresh**. Each entry retains its own topic version and snapshot. Use the original references and Core status to check current facts.

Derived notes retain original references; generated topics are excluded from fact collection.

```json
[
  {
    "id": "0565fc6364f75668158064979761cad4aa2c67c0e7bc7cde052953811268ad7b",
    "sourceClass": "derivedConvention",
    "notes": "T19-audited-purchase 调查约定（audited acceptance model，非原始 synes 业务）：\\n1) 工作流声明源为仓库 business 的 rules.md（sourceClass=documentation），采用小型文本语法：形如 `STATE -> STATE: trigger, guard` 的行表示一条 transitions_to；正则仅三个捕获组（from、to、rest），rest 按第一个逗号分成 trigger 与 guard，原文保留不作解释。当前来源记录的迁移共 6 条（DRAFT->SUBMITTED, SUBMITTED->APPROVED, SUBMITTED->DRAFT, APPROVED->RECEIVING, RECEIVING->COMPLETED, APPROVED->CANCELLED）。文档中的冲突变体句（grossReceived）、日志调用顺序句和 /help 导航句不构成迁移。\\n2) Documents 链：行 `Documents purchase -> receipt -> return use ordered keys tenant_id, purchase_no, line_no.` 直接解析出 documentType 实体（purchase、receipt、return）及 relates_document 边，qualifiers.orderedKeys = [tenant_id, purchase_no, line_no]（键名原样保留）。\\n3) 实体身份：workflowState slots = scopeId, workflowId, semanticKey；documentType slots = scopeId, semanticKey。scopeId 取 configSubset 派生的 repo.business；projectId 取 configSubset.projectId（T19-audited-purchase）；本文档选定稳定 workflowId = audited-acceptance-model。\\n4) flow.ts 是冻结的 T06 可执行工作流样本（code），佐证同一状态词汇与条件（SUBMITTED/APPROVED/RECEIVING/COMPLETED/DRAFT/CANCELLED 及 orderedQty>0&&supplierId、role buyer&&approvalGranted、POSTED not-deleted qty>0、netReceived>=orderedQty&&allReceiptsValid、applicant===actor.id、netReceived===0）；collector 不解析 TS/SQL。\\n5) 证据：小文档允许每文件一条整文件证据（startByte=0, endByte=Buffer.byteLength(content), startLine=1, endLine 按最后非空字节行）；每个实体与关系断言必须引用 output.evidence 内的证据对象。\\n6) 空输入/来源删除时输出成功空替换；dependencies 输出 []，由 Core 记录宿主来源依赖；coverage capability=workflow，sourceSetId 取输入 sourceSets 的 selectorId。",
    "sourceReferences": [
      {
        "snapshot": {
          "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
          "generation": "1",
          "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
        },
        "evidenceId": "ev_e9c5f034613e18fa0025b6a3659d70078d7ec47258e717980285034f0faa8e82",
        "sourceId": "src_0f7166c8fe49c44014a8401875c46def903637df30ebdc47805ee50182d687fa",
        "contentHash": "47228846febce8fe3ea2a13fd1e3ddfad9ae3b668a755e1dae80b80afed57fdc",
        "startByte": 0,
        "endByte": 1057
      },
      {
        "snapshot": {
          "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
          "generation": "1",
          "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
        },
        "evidenceId": "ev_8599bf77bb71ea343d0475d6dde8844c2aca13ad14231f82fcc91948ab71a089",
        "sourceId": "src_f4be1ceeaabcebaae684900161fc5eefbf24bd70e7f0d77a61cdd1bee2b64fac",
        "contentHash": "3b8fa55c4a63cb3695bc33037235b7025b7628b40219af452288d021f1fb7af2",
        "startByte": 0,
        "endByte": 947
      }
    ],
    "configHash": "570279f2876ed2dbefc42e22a28b37884bc52cfcb0caef56c0180f2661b107a2"
  },
  {
    "id": "7ef92757eb63f98a2b67cfac7e306e9ac7fe3c360e006c5d6554e65865349bea",
    "sourceClass": "derivedConvention",
    "notes": "Project T19-audited-purchase, repository `business` (scope repo.business).\n\nRoute/API/frontend/database conventions: not applicable to this snapshot — the selected source set contains only rules.md (documentation) and flow.ts (code); no router registry, HTTP/GraphQL registration, request wrapper, service or dataSource declaration exists in these two files. capabilities api/database/frontend/routes remain not_configured (per status coverage), so no identityDelta is registered.\n\nWorkflow evidence confirmed from current sources (generation 2, snapshot snap_294b38b3...):\n1. rules.md declares a synthetic audited acceptance state machine with explicit transition lines `<FROM> -> <TO>: <trigger>, <guard>` (observed structure: regex-shaped lines DRAFT->SUBMITTED, SUBMITTED->APPROVED, SUBMITTED->DRAFT, APPROVED->RECEIVING, RECEIVING->COMPLETED, APPROVED->CANCELLED). Source: rules.md lines 7-12, hash 47228846...\n2. rules.md declares an ordered document chain \"purchase -> receipt -> return\" with ordered keys tenant_id, purchase_no, line_no (line 13). Document-key vocabulary is declared, not observable in code.\n3. rules.md declares UI page ordering /purchase/approve precedes /Other/purchaseLedger precedes /purchase/receive, and that /help is NOT a business successor (lines 14-15). No route registry exists in the snapshot to corroborate these pages, so they must stay declared-only.\n4. rules.md explicitly forbids treating \"logging before saving\" as a transition and requires gap WORKFLOW_EVIDENCE_MISSING (line 17); flow.ts line 2 comment restates this (\"calling audit before save is not a business transition\"). Call order must never become transitions_to.\n5. rules.md declares a conflict variant (line 18): a document allows finish at grossReceived >= orderedQty while code requires netReceived. Resolution rule: code wins for observed behaviour; the document divergence stays recorded as a declared conflict, not merged into one guard.\n6. flow.ts implements the guards in code: submit requires orderedQty > 0 AND supplierId (line 3); approve requires actor.role === 'buyer' AND approvalGranted (line 8); receive requires sum of POSTED, non-deleted receipt qty > 0 (line 12); finish requires netReceived >= orderedQty AND allReceiptsValid (line 15); withdraw requires state === 'SUBMITTED' AND applicant === actor.id (line 18); cancel requires state === 'APPROVED' AND netReceived === 0 (line 21). These are observed declarations, basis=declared origin=static for the doc-derived states, and the code lines corroborate the same guard vocabulary.\n7. Identity: projectId T19-audited-purchase, scopeId repo.business, workflowId audited-acceptance-model; workflowState slots [scopeId, workflowId, semanticKey] and documentType slots [scopeId, semanticKey] per Core schema=identity. Evidence for file-level claims uses role \"declared-document\" over bytes 0..byteLength.\n8. The ACTIVE plugin bundle t19-audited.workflow@0.1.0 already ships its own identity.mjs, so candidates must reuse it (includeIdentityHelper=false) rather than rewrite the helper. Current sources are byte-identical to the accepted fixture (contentHashes 47228846... and 3b8fa55c...), so a version bump alone preserves every previously verified extraction result.",
    "sourceReferences": [
      {
        "snapshot": {
          "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
          "generation": "2",
          "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
        },
        "evidenceId": "ev_e9c5f034613e18fa0025b6a3659d70078d7ec47258e717980285034f0faa8e82",
        "sourceId": "src_0f7166c8fe49c44014a8401875c46def903637df30ebdc47805ee50182d687fa",
        "contentHash": "47228846febce8fe3ea2a13fd1e3ddfad9ae3b668a755e1dae80b80afed57fdc",
        "startByte": 0,
        "endByte": 1057
      },
      {
        "snapshot": {
          "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
          "generation": "2",
          "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
        },
        "evidenceId": "ev_8599bf77bb71ea343d0475d6dde8844c2aca13ad14231f82fcc91948ab71a089",
        "sourceId": "src_f4be1ceeaabcebaae684900161fc5eefbf24bd70e7f0d77a61cdd1bee2b64fac",
        "contentHash": "3b8fa55c4a63cb3695bc33037235b7025b7628b40219af452288d021f1fb7af2",
        "startByte": 0,
        "endByte": 947
      }
    ],
    "configHash": "570279f2876ed2dbefc42e22a28b37884bc52cfcb0caef56c0180f2661b107a2"
  },
  {
    "id": "99c2d316e1f52d25c65787706025ec99416570cb044c090ab2a2fca7782f89b8",
    "sourceClass": "derivedConvention",
    "notes": "Investigation of T19-audited-purchase (generation 12, snap_294b38b3...):\n\nROUTE / ENTRY: No framework registry observed. flow.ts declares exported module-level functions (submit, approve, receive, finish, withdraw, cancel) as the effective business entrypoints; each is an `operation`-shaped symbol with explicit guard + state assignment in the same function body (src_f4be1ce..., lines 1-22).\n\nBACKEND STATE TRANSITIONS (observed in code, basis=observed):\n- submit -> SUBMITTED guarded by order.orderedQty > 0 && order.supplierId (L3-4).\n- approve -> APPROVED guarded by actor.role === 'buyer' && actor.approvalGranted (L8-9).\n- receive -> RECEIVING guarded by sum over receipts filtered to status==='POSTED' && !deleted of qty > 0 (L12).\n- finish -> COMPLETED guarded by order.netReceived >= order.orderedQty && order.allReceiptsValid (L15). This is the full-file guard expression, not a bare identifier.\n- withdraw -> DRAFT guarded by order.state === 'SUBMITTED' && order.applicant === actor.id (L18).\n- cancel -> CANCELLED guarded by order.state === 'APPROVED' && order.netReceived === 0 (L21).\n\nNON-TRANSITION EVIDENCE (must stay a gap, not an edge): `audit(order)` before `save(order)` inside submit (L2, L5) is call order only; rules.md explicitly marks this WORKFLOW_EVIDENCE_MISSING. Do not derive a transition from statement order.\n\nDOCUMENT/CONFLICT RULE (declared in rules.md): the audited acceptance model requires netReceived for finish; the documented conflict variant allowing grossReceived >= orderedQty is NOT what flow.ts implements — code is authoritative for `observed` claims. keys tenant_id, purchase_no, line_no are declared ordered document keys (rules.md L12).\n\nNAVIGATION (declared in rules.md L13-14): /purchase/approve precedes /Other/purchaseLedger precedes /purchase/receive; /help is not a business successor. These are page-order declarations only, with no corresponding route entities in the current snapshot (code-map returned files=[]), so they remain candidate/gap evidence rather than confirmed relations.\n\nIDENTITY: No scopes/services/dataSources/bindings are verifiable from these two sources alone; registering none in this run (identityDelta omitted). configHash 570279f2876ed2dbefc42e22a28b37884bc52cfcb0caef56c0180f2661b107a2, scopeIds [\"repo.business\"].\n\nCITATION CONVENTION: for guard-level facts use the Core-generated full-file sourceReference (sourceId src_f4be1ceeaabcebaae684900161fc5eefbf24bd70e7f0d77a61cdd1bee2b64fac, contentHash 3b8fa55c..., startByte=0, endByte=947, ev_8599bf77bb71ea343d0475d6dde8844c2aca13ad14231f82fcc91948ab71a089). A six-byte range that covers only the identifier `finish` does not contain the guard expression and must not be used to support guard claims.",
    "sourceReferences": [
      {
        "snapshot": {
          "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
          "generation": "12",
          "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
        },
        "evidenceId": "ev_e9c5f034613e18fa0025b6a3659d70078d7ec47258e717980285034f0faa8e82",
        "sourceId": "src_0f7166c8fe49c44014a8401875c46def903637df30ebdc47805ee50182d687fa",
        "contentHash": "47228846febce8fe3ea2a13fd1e3ddfad9ae3b668a755e1dae80b80afed57fdc",
        "startByte": 0,
        "endByte": 1057
      },
      {
        "snapshot": {
          "indexId": "c009752d-bed4-4681-8b07-7bf57fe02431",
          "generation": "12",
          "sourceSnapshotId": "snap_294b38b32be40c8bda6c123958815e207ab97cefc4a548d3c3c4e088d3b3384f"
        },
        "evidenceId": "ev_8599bf77bb71ea343d0475d6dde8844c2aca13ad14231f82fcc91948ab71a089",
        "sourceId": "src_f4be1ceeaabcebaae684900161fc5eefbf24bd70e7f0d77a61cdd1bee2b64fac",
        "contentHash": "3b8fa55c4a63cb3695bc33037235b7025b7628b40219af452288d021f1fb7af2",
        "startByte": 0,
        "endByte": 947
      }
    ],
    "configHash": "570279f2876ed2dbefc42e22a28b37884bc52cfcb0caef56c0180f2661b107a2"
  }
]
```
