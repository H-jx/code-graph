import { search } from './jobs';
export const moduleInfo = { key: 'purchase', name: '采购模块' };
scheduler.register('nightly-ledger', search);
messages.subscribe('purchase.refresh', search);
commands.register('refresh-ledger', search);
