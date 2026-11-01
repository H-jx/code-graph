# Independent arithmetic audit of the plan's proposed sample

This is a local audited acceptance model; it is not a claim about synes production accounting.
Grain: tenant_id + purchase_no + line_no, restricted to the requested tenant and APPROVED purchases.
Valid receipts/returns/invoices: status POSTED and deleted = 0. Aggregate each child table
at the grain BEFORE LEFT JOIN. No match is zero, not a reason to remove the detail.
grossReceived = receipt quantity sum; netReceived = grossReceived - return quantity sum;
outstanding = orderedQty - netReceived; purchaseAmountCents = orderedQty * unitPriceCents;
invoiceAmountCents = invoice amount sum. Supplier joins require tenant_id + supplier_id.
Two databases ds.erp and ds.analytics contain the same table names; only ds.erp is bound.
SQL runs only in the in-memory acceptance test. No business database is connected.
