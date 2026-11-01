CREATE TABLE purchase_detail (
 tenant_id TEXT, purchase_no TEXT, line_no INTEGER, supplier_id TEXT,
 ordered_qty INTEGER, unit_price_cents INTEGER, status TEXT,
 PRIMARY KEY(tenant_id, purchase_no, line_no));
CREATE TABLE supplier (tenant_id TEXT, supplier_id TEXT, name TEXT, PRIMARY KEY(tenant_id,supplier_id));
CREATE TABLE receipt (id TEXT PRIMARY KEY, tenant_id TEXT, purchase_no TEXT, line_no INTEGER, qty INTEGER, status TEXT, deleted INTEGER,
 FOREIGN KEY(tenant_id,purchase_no,line_no) REFERENCES purchase_detail(tenant_id,purchase_no,line_no));
CREATE TABLE return_line (id TEXT PRIMARY KEY, tenant_id TEXT, purchase_no TEXT, line_no INTEGER, qty INTEGER, status TEXT, deleted INTEGER,
 FOREIGN KEY(tenant_id,purchase_no,line_no) REFERENCES purchase_detail(tenant_id,purchase_no,line_no));
CREATE TABLE invoice (id TEXT PRIMARY KEY, tenant_id TEXT, purchase_no TEXT, line_no INTEGER, amount_cents INTEGER, status TEXT, deleted INTEGER,
 FOREIGN KEY(tenant_id,purchase_no,line_no) REFERENCES purchase_detail(tenant_id,purchase_no,line_no));
