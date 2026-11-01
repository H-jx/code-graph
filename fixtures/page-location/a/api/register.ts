import { ledger } from './service';
http.get('/purchase/ledger', ledger);
graphql.register('Query', 'purchaseSummary', ledger);
