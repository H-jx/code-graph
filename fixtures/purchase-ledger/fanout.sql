SELECT p.purchase_no,p.line_no,SUM(r.qty),SUM(i.amount_cents)
FROM purchase_detail p
LEFT JOIN receipt r ON r.tenant_id=p.tenant_id AND r.purchase_no=p.purchase_no AND r.line_no=p.line_no AND r.status='POSTED' AND r.deleted=0
LEFT JOIN invoice i ON i.tenant_id=p.tenant_id AND i.purchase_no=p.purchase_no AND i.line_no=p.line_no AND i.status='POSTED' AND i.deleted=0
WHERE p.tenant_id=:tenant AND p.status='APPROVED' GROUP BY p.purchase_no,p.line_no;
