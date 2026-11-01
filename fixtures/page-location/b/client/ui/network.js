import { http } from './client';
export function loadRows() { return http.get('/purchase/ledger'); }
export function loadStats() { return http.graphql(`query PurchaseSummary { purchaseSummary { count } }`); }
export function loadDynamic() { return http.get('/purchase/' + globalThis.target); }
