export function ledger() { return erp.query(ledgerSql); }
export const ledgerSql = `SELECT * FROM purchase_detail WHERE tenant_id = :tenant`;
export const dynamicSql = 'SELECT * FROM ' + runtimeTable;
