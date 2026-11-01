from .models import erp, analytics, Purchase
# 中文 🧾 不改变 Python AST 的 UTF-8 字节定位。
def boundLedger():
    return erp.query("SELECT tenant_id, purchase_no, line_no FROM purchase_detail WHERE status = 'APPROVED'\n")

def wrongInstance():
    return analytics.query('SELECT tenant_id FROM purchase_detail')

def modelRows():
    return Purchase.findAll()

def dynamicQuery(table):
    return erp.query('SELECT * FROM ' + table)

def shadowed(erp):
    return erp.query('SELECT * FROM purchase_detail')

def shadowedLocal():
    erp = runtimeDb
    return erp.query('SELECT * FROM purchase_detail')

def unsupportedOrm():
    return erp.raw('SELECT * FROM purchase_detail')
