from .queries import ledger
app.get('/purchase/ledger')(ledger)
graphql.register('Query', 'purchaseSummary', ledger)
