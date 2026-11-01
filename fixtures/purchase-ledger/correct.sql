WITH r AS (
 SELECT tenant_id,purchase_no,line_no,SUM(qty) AS qty FROM receipt
 WHERE status='POSTED' AND deleted=0 GROUP BY tenant_id,purchase_no,line_no
), t AS (
 SELECT tenant_id,purchase_no,line_no,SUM(qty) AS qty FROM return_line
 WHERE status='POSTED' AND deleted=0 GROUP BY tenant_id,purchase_no,line_no
), i AS (
 SELECT tenant_id,purchase_no,line_no,SUM(amount_cents) AS amount FROM invoice
 WHERE status='POSTED' AND deleted=0 GROUP BY tenant_id,purchase_no,line_no
)
SELECT p.tenant_id,p.purchase_no,p.line_no,s.name,
 COALESCE(r.qty,0) AS grossReceived,COALESCE(r.qty,0)-COALESCE(t.qty,0) AS netReceived,
 p.ordered_qty-COALESCE(r.qty,0)+COALESCE(t.qty,0) AS outstanding,
 p.ordered_qty*p.unit_price_cents AS purchaseAmountCents,COALESCE(i.amount,0) AS invoiceAmountCents
FROM purchase_detail p
LEFT JOIN supplier s ON s.tenant_id=p.tenant_id AND s.supplier_id=p.supplier_id
LEFT JOIN r ON r.tenant_id=p.tenant_id AND r.purchase_no=p.purchase_no AND r.line_no=p.line_no
LEFT JOIN t ON t.tenant_id=p.tenant_id AND t.purchase_no=p.purchase_no AND t.line_no=p.line_no
LEFT JOIN i ON i.tenant_id=p.tenant_id AND i.purchase_no=p.purchase_no AND i.line_no=p.line_no
WHERE p.tenant_id=:tenant AND p.status='APPROVED' ORDER BY p.purchase_no,p.line_no;
