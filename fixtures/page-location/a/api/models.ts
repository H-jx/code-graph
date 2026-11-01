export const erp = orm.connect('ds.erp');
export const analytics = orm.connect('ds.analytics');
export const Purchase = erp.model('purchase_detail', ['tenant_id', 'purchase_no', 'line_no']);
export const OtherPurchase = analytics.model('purchase_detail', ['tenant_id', 'purchase_no', 'line_no']);
