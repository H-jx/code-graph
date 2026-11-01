erp = orm.connect('ds.erp')
analytics = orm.connect('ds.analytics')
Purchase = erp.model('purchase_detail', ['tenant_id', 'purchase_no', 'line_no'])
OtherPurchase = analytics.model('purchase_detail', ['tenant_id', 'purchase_no', 'line_no'])
