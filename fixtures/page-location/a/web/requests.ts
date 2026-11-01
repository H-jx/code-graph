import { http } from './transport';
export function fetchLedger() { return http.get('/purchase/ledger'); }
export function fetchSummary() { return http.graphql(`query PurchaseSummary { purchaseSummary { count } }`); }
export function dynamicRequest() { return http.get('/purchase/' + globalThis.target); }
