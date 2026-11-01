SELECT p.purchase_no,p.line_no,SUM(r.qty)
FROM purchase_detail p LEFT JOIN receipt r
ON r.tenant_id=p.tenant_id AND r.purchase_no=p.purchase_no AND r.line_no=p.line_no
WHERE p.tenant_id=:tenant AND p.status='APPROVED' AND r.status='POSTED' AND r.deleted=0
GROUP BY p.purchase_no,p.line_no;
