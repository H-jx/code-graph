import { erp, analytics, Purchase } from './models';
// 原文包含中文与转义；证据引用完整字符串，不假造 SQL token 偏移。
export function boundLedger() {
  return erp.query("SELECT tenant_id, purchase_no, line_no FROM purchase_detail WHERE status = 'APPROVED'\n");
}
export function wrongInstance() { return analytics.query('SELECT tenant_id FROM purchase_detail'); }
export function modelRows() { return Purchase.findAll(); }
export function dynamicQuery(table) { return erp.query('SELECT * FROM ' + table); }
export function shadowed(erp) { return erp.query('SELECT * FROM purchase_detail'); }
export function shadowedLocal() { const erp = runtimeDb; return erp.query('SELECT * FROM purchase_detail'); }
export function unsupportedOrm() { return erp.raw('SELECT * FROM purchase_detail'); }
