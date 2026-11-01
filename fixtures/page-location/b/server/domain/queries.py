def ledger():
    return erp.query("SELECT * FROM purchase_detail WHERE tenant_id = :tenant")

def dynamic_query(table):
    return erp.query("SELECT * FROM " + table)
