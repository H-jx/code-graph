export function submit(order) {
  audit(order); // calling audit before save is not a business transition
  if (!(order.orderedQty > 0 && order.supplierId)) throw Error('invalid order');
  order.state = 'SUBMITTED';
  save(order);
}
export function approve(order, actor) {
  if (actor.role !== 'buyer' || !actor.approvalGranted) throw Error('not approved');
  order.state = 'APPROVED';
}
export function receive(order, receipts) {
  if (receipts.filter(r => r.status === 'POSTED' && !r.deleted).reduce((n, r) => n + r.qty, 0) > 0) order.state = 'RECEIVING';
}
export function finish(order) {
  if (order.netReceived >= order.orderedQty && order.allReceiptsValid) order.state = 'COMPLETED';
}
export function withdraw(order, actor) {
  if (order.state === 'SUBMITTED' && order.applicant === actor.id) order.state = 'DRAFT';
}
export function cancel(order) {
  if (order.state === 'APPROVED' && order.netReceived === 0) order.state = 'CANCELLED';
}
