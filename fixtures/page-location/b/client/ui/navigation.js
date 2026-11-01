export const router = { id: 'workspace', mode: 'history' };
export const screens = [{ url: '/Other/purchaseLedger', alias: '/purchase', labelKey: 'purchase.title', load: () => import('./views/Board') }];
