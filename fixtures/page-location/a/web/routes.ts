import Ledger from './pages/Ledger';
export const router = { id: 'main', mode: 'hash' };
export const routes = [{ path: '/Other/purchaseLedger', alias: '/purchase', titleKey: 'purchase.title', component: Ledger }];
