# Audited acceptance model, not the synes business model

Source: NEW-PROJECT-LONG-TASK.md section 5; frozen before plugin development.
The state vocabulary below is synthetic. The original synes vocabulary is kept separately.

DRAFT -> SUBMITTED: submit, orderedQty > 0 AND supplierId present.
SUBMITTED -> APPROVED: approve, role == buyer AND approvalGranted.
SUBMITTED -> DRAFT: reject or withdraw, applicant == actor for withdraw.
APPROVED -> RECEIVING: receive, sum(POSTED, not-deleted receipt qty) > 0.
RECEIVING -> COMPLETED: finish, netReceived >= orderedQty AND all receipts valid.
APPROVED -> CANCELLED: cancel, netReceived == 0.
Documents purchase -> receipt -> return use ordered keys tenant_id, purchase_no, line_no.
UI pages: /purchase/approve precedes /Other/purchaseLedger, which precedes /purchase/receive.
The navigation /help is not a business successor.
Logging before saving is only a call order, with WORKFLOW_EVIDENCE_MISSING.
Conflict variant: a document allows finish at grossReceived >= orderedQty while code requires netReceived.
