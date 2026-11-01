import { query } from './workers';
export const moduleInfo = { key: 'purchase', name: '采购模块' };
scheduler.register('nightly-ledger', query);
messages.subscribe('purchase.refresh', query);
commands.register('refresh-ledger', query);
